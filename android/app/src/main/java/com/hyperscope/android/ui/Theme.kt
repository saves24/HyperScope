package com.hyperscope.android.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import android.os.Build

// Kept for the (removed) translucent-card mode; all cards use 3dp by default.
val LocalCardElevation = staticCompositionLocalOf { 3.dp }

// Palette mirrors the web panel (style.css): light #f1f5f9 / dark #0f172a,
// primary indigo #4338ca.

private val LightColors = lightColorScheme(
    primary = Color(0xFF4338CA),
    onPrimary = Color.White,
    background = Color(0xFFF1F5F9),
    onBackground = Color(0xFF0F172A),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF0F172A),
    surfaceVariant = Color(0xFFF8FAFC),
    outline = Color(0xFFDDDDDD),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFF818CF8),
    onPrimary = Color(0xFF0F172A),
    background = Color(0xFF0F172A),
    onBackground = Color(0xFFE2E8F0),
    surface = Color(0xFF1E293B),
    onSurface = Color(0xFFE2E8F0),
    surfaceVariant = Color(0xFF0F172A),
    outline = Color(0xFF334155),
)

/**
 * Theme wrapper.
 */
@Composable
fun HyperScopeTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit,
) {
    // Material You: on Android 12+ use the system wallpaper palette when the
    // user picked "auto" theme — the app then matches the device accent colors.
    val colors = if (dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        if (darkTheme) dynamicDarkColorScheme(LocalContext.current)
        else dynamicLightColorScheme(LocalContext.current)
    } else if (darkTheme) DarkColors else LightColors

    MaterialTheme(colorScheme = colors) {
        CompositionLocalProvider(
            LocalCardElevation provides 3.dp
        ) {
            Box(Modifier.fillMaxSize()) {
                content()
            }
        }
    }
}
