package com.hyperscope.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.hyperscope.android.data.TrendHistory
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

/**
 * Semi-circular "speed gauge" card (web-panel style): arc + needle + big value.
 * Used for CPU / memory on the dashboard.
 */
@Composable
fun GaugeCard(
    title: String,
    percent: Double,
    valueLabel: String,
    modifier: Modifier = Modifier,
    subLabel: String? = null,
) {
    Card(modifier = modifier, shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = LocalCardElevation.current)) {
        Column(Modifier.fillMaxWidth().padding(10.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
            GaugeArc(percent = percent, color = gaugeColor(percent))
            // Big value sits below the arc (not overlapping it).
            Text(valueLabel, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
            subLabel?.let {
                Text(it, style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
            }
        }
    }
}

/** Draws the semi-circular arc + needle for a gauge. */
@Composable
private fun GaugeArc(percent: Double, color: Color) {
    val pct = percent.coerceIn(0.0, 100.0)
    Canvas(modifier = Modifier.fillMaxWidth().height(70.dp)) {
        val stroke = 11.dp.toPx()
        val w = size.width
        val h = size.height
        val radius = min(w / 2 - stroke, h - stroke)
        val center = Offset(w / 2, h)
        val start = 180f
        val sweep = 180f

        // background arc
        drawArc(
            color = color.copy(alpha = 0.18f),
            startAngle = start,
            sweepAngle = sweep,
            useCenter = false,
            topLeft = Offset(center.x - radius, center.y - radius),
            size = Size(radius * 2, radius * 2),
            style = Stroke(width = stroke, cap = StrokeCap.Round),
        )
        // value arc
        val valueSweep = (sweep * pct / 100.0).toFloat()
        drawArc(
            color = color,
            startAngle = start,
            sweepAngle = valueSweep,
            useCenter = false,
            topLeft = Offset(center.x - radius, center.y - radius),
            size = Size(radius * 2, radius * 2),
            style = Stroke(width = stroke, cap = StrokeCap.Round),
        )
        // needle
        val angleRad = Math.toRadians(180.0 - valueSweep)
        val needleLen = radius - stroke / 2
        val tip = Offset(
            center.x + (needleLen * cos(angleRad)).toFloat(),
            center.y - (needleLen * sin(angleRad)).toFloat(),
        )
        drawLine(
            color = color,
            start = Offset(center.x, center.y),
            end = tip,
            strokeWidth = 3.dp.toPx(),
            cap = StrokeCap.Round,
        )
    }
}

private fun gaugeColor(pct: Double): Color = when {
    pct > 85 -> Color(0xFFDC2626)
    pct > 70 -> Color(0xFFF59E0B)
    else -> Color(0xFF4338CA)
}

/** Trend line chart card (web-panel style speed trend). */
@Composable
fun TrendCard(
    title: String,
    history: TrendHistory,
    modifier: Modifier = Modifier,
) {
    val cpu = history.cpu
    val mem = history.mem
    Card(modifier = modifier, shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = LocalCardElevation.current)) {
        Column(Modifier.fillMaxWidth().padding(12.dp)) {
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
            Spacer(Modifier.height(6.dp))
            TrendChart(cpu = cpu, mem = mem)
            Spacer(Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                LegendDot(Color(0xFF4338CA)); Spacer(Modifier.width(4.dp))
                Text("CPU", style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                Spacer(Modifier.width(12.dp))
                LegendDot(Color(0xFF22C55E)); Spacer(Modifier.width(4.dp))
                Text("MEM", style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
            }
        }
    }
}

@Composable
private fun TrendChart(cpu: List<Double>, mem: List<Double>) {
    Canvas(modifier = Modifier.fillMaxWidth().height(72.dp)) {
        val maxPoints = maxOf(cpu.size, mem.size)
        if (maxPoints < 2) return@Canvas
        val padX = 4.dp.toPx()
        val padY = 6.dp.toPx()
        val chartW = size.width - padX * 2
        val chartH = size.height - padY * 2

        fun drawSeries(values: List<Double>, color: Color) {
            if (values.size < 2) return
            val stepX = chartW / (values.size - 1)
            val points = values.mapIndexed { i, v ->
                val y = padY + chartH - ((v.coerceIn(0.0, 100.0) / 100.0) * chartH).toFloat()
                Offset(padX + i * stepX, y)
            }
            for (i in 0 until points.size - 1) {
                drawLine(color, points[i], points[i + 1], strokeWidth = 2.dp.toPx())
            }
        }
        drawSeries(cpu, Color(0xFF4338CA))
        drawSeries(mem, Color(0xFF22C55E))
    }
}

@Composable
private fun LegendDot(color: Color) {
    Canvas(modifier = Modifier.width(8.dp).height(8.dp)) {
        drawCircle(color)
    }
}
