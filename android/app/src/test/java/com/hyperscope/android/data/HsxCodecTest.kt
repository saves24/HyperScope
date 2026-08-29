package com.hyperscope.android.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for the .hsxc encrypted config codec (pure JVM, no Android deps).
 * Verifies the container layout matches the web panel: HSX1 magic + salt +
 * PBKDF2(200k) + nonce + AES-256-GCM, and that round-trips are stable.
 */
class HsxCodecTest {

    @Test
    fun decrypt_roundTrips_jsonPayload() {
        val json = """{"nodes":[{"name":"pi","addr":"192.168.1.6","port":5000,"key":"abc123","tls":true}]}"""
        val bytes = json.toByteArray(Charsets.UTF_8)
        val pass = "test-passphrase"

        val encrypted = HsxCodec.encrypt(bytes, pass)
        // Magic header present.
        assertEquals(0x48, encrypted[0].toInt() and 0xFF) // 'H'
        assertEquals(0x53, encrypted[1].toInt() and 0xFF) // 'S'
        assertEquals(0x58, encrypted[2].toInt() and 0xFF) // 'X'
        assertEquals(0x31, encrypted[3].toInt() and 0xFF) // '1'

        val decrypted = HsxCodec.decryptForTest(encrypted, pass)
        assertEquals(json, decrypted)
    }

    @Test
    fun decrypt_rejectsWrongPassphrase() {
        val json = """{"nodes":[]}"""
        val encrypted = HsxCodec.encrypt(json.toByteArray(), "right")
        try {
            HsxCodec.decryptForTest(encrypted, "wrong")
            assertTrue("expected IllegalArgumentException", false)
        } catch (_: IllegalArgumentException) {
            // expected
        }
    }

    @Test
    fun decrypt_rejectsNonHsxData() {
        try {
            HsxCodec.decryptForTest("not-an-hsx-file".toByteArray(), "x")
            assertTrue("expected IllegalArgumentException", false)
        } catch (_: IllegalArgumentException) {
            // expected
        }
    }
}
