package com.hyperscope.android.data

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.Signature

/**
 * Device identity for the relay protocol: an Ed25519 key pair generated in
 * Android Keystore (hardware-backed where available). The private key never
 * leaves the Keystore; only the public key is shared with collectors so they
 * can verify this device's command signatures.
 */
object DeviceIdentity {
    private const val ALIAS = "hyperscope-device-key"
    private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    private var cachedPub: String? = null

    /** True when this device already has a key. */
    fun exists(): Boolean = try {
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        ks.containsAlias(ALIAS)
    } catch (e: Exception) {
        false
    }

    /** Create the device key if missing. Returns the base64 public key. */
    fun ensureKey(): String {
        if (!exists()) {
            // "ED25519" is the algorithm name; KeyProperties.KEY_ALGORITHM_ED25519
            // exists only on API 28+ (minSdk is 28), but referencing the
            // constant directly can fail to resolve on some build setups, so
            // use the literal algorithm name.
            val generator = KeyPairGenerator.getInstance(
                "ED25519", ANDROID_KEYSTORE
            )
            val spec = KeyGenParameterSpec.Builder(
                ALIAS, KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
            )
                .setDigests(KeyProperties.DIGEST_NONE)
                .build()
            generator.initialize(spec)
            generator.generateKeyPair()
        }
        return publicKeyB64()
    }

    /**
     * Base64-encoded raw Ed25519 public key (32 bytes). The collector's
     * verify_device_signature expects the raw key, not X.509-wrapped.
     */
    fun publicKeyB64(): String = try {
        cachedPub?.let { return it }
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val entry = ks.getEntry(ALIAS, null) as KeyStore.PrivateKeyEntry
        val encoded = entry.certificate.publicKey.encoded
        // Ed25519 SubjectPublicKeyInfo: 12-byte header + 32-byte key.
        val raw = if (encoded.size == 44) encoded.copyOfRange(12, 44) else encoded
        Base64.encodeToString(raw, Base64.NO_WRAP).also { cachedPub = it }
    } catch (e: Exception) {
        ""
    }

    /**
     * Sign a message with the device key (Ed25519). The signed message must
     * match what the collector reconstructs (e.g. "action:deviceId").
     */
    fun sign(message: String): String? = try {
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val entry = ks.getEntry(ALIAS, null) as KeyStore.PrivateKeyEntry
        val sig = Signature.getInstance("Ed25519")
        sig.initSign(entry.privateKey)
        sig.update(message.toByteArray(Charsets.UTF_8))
        Base64.encodeToString(sig.sign(), Base64.NO_WRAP)
    } catch (e: Exception) {
        null
    }

    /**
     * Sign a message with a raw 32-byte Ed25519 private key (base64-encoded).
     * Used when a shared panel identity was imported from a .hsxc config: the
     * client signs as "panel" so nodes that already trust the panel accept it.
     */
    fun signWithKey(keyB64: String, message: String): String? = try {
        val raw = Base64.decode(keyB64, Base64.NO_WRAP)
        val spec = java.security.spec.PKCS8EncodedKeySpec(
            byteArrayOf(0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20).plus(raw)
        )
        val kf = KeyFactory.getInstance("Ed25519")
        val privateKey = kf.generatePrivate(spec)
        val sig = Signature.getInstance("Ed25519")
        sig.initSign(privateKey)
        sig.update(message.toByteArray(Charsets.UTF_8))
        Base64.encodeToString(sig.sign(), Base64.NO_WRAP)
    } catch (e: Exception) {
        null
    }
}
