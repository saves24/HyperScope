package com.hyperscope.android.data

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import javax.crypto.Cipher
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.PBEKeySpec
import javax.crypto.spec.SecretKeySpec

/**
 * Codec for the HyperScope .hsxc node-config file.
 *
 * File layout (matches the web panel exactly):
 *   bytes 0..3     magic "HSX1"
 *   bytes 4..19    PBKDF2 salt (16)
 *   bytes 20..31   AES-GCM IV (12)
 *   bytes 32..     AES-256-GCM ciphertext (incl. 16-byte auth tag)
 * Plaintext inside: {"nodes":[{name,addr,port,key,tls}]}
 * Decryption is fully local (Java standard crypto), nothing leaves the phone.
 */
object HsxCodec {
    private const val MAGIC = "HSX1"
    private const val ITERATIONS = 200000
    private const val KEY_LEN = 256
    private val json = Json { ignoreUnknownKeys = true }
    /** Shared Json instance (used by the exporter to serialize payloads). */
    val jsonExport: Json get() = json

    @Serializable
    data class HsxNode(
        val name: String = "",
        val addr: String = "",
        val port: Int = 8686,
        val key: String = "",
        val tls: Boolean = false,
        val push: Boolean = true,
        val node_pubkey: String = "",
    )

    @Serializable
    data class HsxPayload(
        val nodes: List<HsxNode> = emptyList(),
        // Shared panel identity private key (base64, 32 raw bytes). Imported
        // clients sign commands as "panel" so nodes that already trust the
        // panel accept them without per-device setup.
        val identity_key: String = "",
    )

    /** Validates the file header; returns true if it is a HyperScope .hsxc file. */
    fun isHsx(data: ByteArray): Boolean {
        if (data.size < 32) return false
        val head = data.copyOfRange(0, 4).toString(Charsets.US_ASCII)
        return head == MAGIC
    }

    /**
     * Decrypts an .hsxc file with the given passphrase.
     * Layout: "HSX1"(4) + salt(16) + iv(12) + ciphertext + authTag(16).
     * @throws IllegalArgumentException on bad magic, wrong passphrase or corruption.
     */
    fun decrypt(data: ByteArray, passphrase: String): HsxPayload {
        val pt = decryptRaw(data, passphrase)
        return runCatching {
            json.decodeFromString(HsxPayload.serializer(), String(pt, Charsets.UTF_8))
        }.getOrElse { throw IllegalArgumentException("Wrong passphrase or corrupted file") }
    }

    /** Re-derives the AES-256 key from the passphrase via PBKDF2-HMAC-SHA256. */
    private fun deriveKey(passphrase: String, salt: ByteArray): ByteArray {
        val spec = PBEKeySpec(passphrase.toCharArray(), salt, ITERATIONS, KEY_LEN)
        val factory = SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256")
        return factory.generateSecret(spec).encoded
    }

    /**
     * Encrypts a JSON payload into the .hsxc layout. Used for config export;
     * the web panel and Android share this exact layout.
     */
    fun encrypt(plaintext: ByteArray, passphrase: String): ByteArray {
        val salt = ByteArray(16).also { java.security.SecureRandom().nextBytes(it) }
        val iv = ByteArray(12).also { java.security.SecureRandom().nextBytes(it) }
        val key = deriveKey(passphrase, salt)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
        val ct = cipher.doFinal(plaintext)
        return MAGIC.toByteArray(Charsets.US_ASCII) + salt + iv + ct
    }

    /** Test-only decrypt that returns the raw plaintext JSON string. */
    internal fun decryptForTest(data: ByteArray, passphrase: String): String {
        return String(decryptRaw(data, passphrase), Charsets.UTF_8)
    }

    private fun decryptRaw(data: ByteArray, passphrase: String): ByteArray {
        if (!isHsx(data)) throw IllegalArgumentException("Not a valid .hsxc file")
        val salt = data.copyOfRange(4, 20)
        val iv = data.copyOfRange(20, 32)
        val ctFull = data.copyOfRange(32, data.size)
        val key = deriveKey(passphrase, salt)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
        return try {
            cipher.doFinal(ctFull)
        } catch (e: Exception) {
            throw IllegalArgumentException("Wrong passphrase or corrupted file")
        }
    }
}
