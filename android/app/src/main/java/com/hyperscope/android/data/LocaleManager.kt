package com.hyperscope.android.data

import android.content.Context
import android.content.res.Configuration
import java.util.Locale

/**
 * Applies the stored language preference ("system" | "zh" | "en") to the
 * current context's configuration. Called from the Activity before setContent,
 * and re-applied after a language switch.
 */
object LocaleManager {
    fun applyLocale(context: Context, lang: String): Context {
        val locale = when (lang) {
            "zh" -> Locale.SIMPLIFIED_CHINESE
            "en" -> Locale.ENGLISH
            else -> Locale.getDefault() // follow device
        }
        Locale.setDefault(locale)
        val config = Configuration(context.resources.configuration)
        config.setLocale(locale)
        return context.createConfigurationContext(config)
    }
}
