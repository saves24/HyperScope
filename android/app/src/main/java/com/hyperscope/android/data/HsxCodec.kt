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

    @Serializable
    data class HsxNode(val name: String = "", val addr: String = "", val port: Int = 5000, val key: String = "", val tls: Boolean = false)

    @Serializable
    data class HsxPayload(val nodes: List<HsxNode> = emptyList())

    /** Validates the file header; returns true if it is a HyperScope .hsxc file. */
    fun isHsx(data: ByteArray): Boolean {
        if (data.size < 32) return false
        val head = data.copyOfRange(0, 4).toString(Charsets.US_ASCII)
        return head == MAGIC
    }

    /**
     * Decrypts an .hsxc file with the given passphrase.
     * @throws IllegalArgumentException on bad magic, wrong passphrase or corruption.
     */
    fun decrypt(data: ByteArray, passphrase: String): HsxPayload {
        if (!isHsx(data)) throw IllegalArgumentException("Not a valid .hsxc file")
        val salt = data.copyOfRange(4, 20)
        val iv = data.copyOfRange(20, 32)
        val ct = data.copyOfRange(32, data.size)
        val key = deriveKey(passphrase, salt)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
        val pt = try {
            cipher.doFinal(ct)
        } catch (e: Exception) {
            throw IllegalArgumentException("Wrong passphrase or corrupted file")
        }
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

    // ---- Encoding helpers (export kept local too; symmetric with the web panel) ----

    fun encrypt(passphrase: String, payload: HsxPayload): ByteArray {
        val salt = ByteArray(16).also { java.security.SecureRandom().nextBytes(it) }
        val iv = ByteArray(12).also { java.security.SecureRandom().nextBytes(it) }
        val key = deriveKey(passphrase, salt)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
        val ct = cipher.doFinal(json.encodeToString(HsxPayload.serializer(), payload).toByteArray(Charsets.UTF_8))
        val out = ByteArray(32 + ct.size)
        System.arraycopy(MAGIC.toByteArray(Charsets.US_ASCII), 0, out, 0, 4)
        System.arraycopy(salt, 0, out, 4, 16)
        System.arraycopy(iv, 0, out, 20, 12)
        System.arraycopy(ct, 0, out, 32, ct.size)
        return out
    }
}
