package com.hyperscope.android.ui

import android.app.Activity
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import com.hyperscope.android.R

private val languages = listOf(
    "system" to R.string.auth_system_lang,
    "zh" to R.string.lang_zh,
    "en" to R.string.lang_en,
    "ru" to R.string.lang_ru,
)

/**
 * Dropdown to pick the app language ("system" follows the device language).
 * Recreate is required so Android re-inflates resources with the new locale.
 */
@Composable
fun LanguagePicker(lang: String, onSelect: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    val context = LocalContext.current
    OutlinedButton(onClick = { expanded = true }) {
        Text(stringResource(R.string.language) + ": " +
            stringResource(languages.firstOrNull { it.first == lang }?.second ?: R.string.auth_system_lang))
    }
    DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
        languages.forEach { (value, labelRes) ->
            DropdownMenuItem(text = { Text(stringResource(labelRes)) }, onClick = {
                expanded = false
                onSelect(value)
                (context as? Activity)?.recreate()
            })
        }
    }
}
