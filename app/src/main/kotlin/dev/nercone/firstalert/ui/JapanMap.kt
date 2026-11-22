package dev.nercone.firstalert.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.rememberTransformableState
import androidx.compose.foundation.gestures.transformable
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nercone.firstalert.core.Coordinate
import dev.nercone.firstalert.core.Intensity
import dev.nercone.firstalert.core.MapShape
import dev.nercone.firstalert.core.Waves
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min

/**
 * A map of Japan, drawn from the outlines of the prefectures the core holds (Natural Earth).
 *
 * The projection is plate carrée scaled by the cosine of 36°N, which keeps the shape of Japan
 * right at the scale of a phone. Prefectures are filled with the colour of their intensity; the
 * epicentre is a cross (a shape, not only a colour), the user's place a ringed dot, and the P and
 * S waves of an active warning are circles (S filled, P only outlined). The map can be zoomed and
 * moved (one finger moves it only once zoomed in, so that the screen still scrolls over it), and tapped (`onTap`) for choosing a place.
 */
object JapanMap {
    private const val REFERENCE_LATITUDE = 36.0
    private val COS = cos(REFERENCE_LATITUDE * PI / 180).toFloat()

    /** The area shown when nothing else is: the main islands and Okinawa. */
    val JAPAN = Area(24.0, 122.5, 46.0, 146.5)

    data class Area(val south: Double, val west: Double, val north: Double, val east: Double) {
        /** An area of `radiusKm` around `centre`. */
        companion object {
            fun around(centre: Coordinate, radiusKm: Double): Area {
                val dLat = radiusKm / 111.2
                val dLon = dLat / cos(centre.latitude * PI / 180)
                return Area(centre.latitude - dLat, centre.longitude - dLon, centre.latitude + dLat, centre.longitude + dLon)
            }
        }
    }

    fun x(longitude: Double): Float = (longitude * COS).toFloat()

    fun y(latitude: Double): Float = (-latitude).toFloat()

    /** The paths of the prefectures in projected units, by prefecture code. */
    fun paths(shapes: List<MapShape>): Map<Int, Path> = shapes.associate { shape ->
        shape.prefecture to Path().apply {
            for (ring in shape.rings) {
                ring.forEachIndexed { index, point ->
                    if (index == 0) moveTo(x(point[0]), y(point[1])) else lineTo(x(point[0]), y(point[1]))
                }
                close()
            }
        }
    }

    @Composable
    fun Content(
        shapes: List<MapShape>,
        fills: Map<Int, Intensity>,
        epicentre: Coordinate?,
        place: Coordinate?,
        waves: List<Waves>,
        area: Area,
        description: String,
        modifier: Modifier = Modifier,
        aspect: Float = 1f,
        onTap: ((Coordinate) -> Unit)? = null,
    ) {
        val paths = remember(shapes) { paths(shapes) }
        var zoom by remember(area) { mutableFloatStateOf(1f) }
        var pan by remember(area) { mutableStateOf(Offset.Zero) }
        var canvas by remember { mutableStateOf(Size.Zero) }
        // Zooms about the point between the fingers: the point under them stays under them.
        val transform = rememberTransformableState { centroid, zoomChange, panChange, _ ->
            val next = (zoom * zoomChange).coerceIn(0.5f, 12f)
            val change = next / zoom
            val half = Offset(canvas.width / 2, canvas.height / 2)
            pan = centroid - half - (centroid - half - pan) * change + panChange
            zoom = next
        }
        val land = MaterialTheme.colorScheme.surfaceContainerHighest
        val sea = MaterialTheme.colorScheme.surfaceContainerLow
        val border = MaterialTheme.colorScheme.outline
        val marker = MaterialTheme.colorScheme.onSurface
        val placeColor = MaterialTheme.colorScheme.primary
        Canvas(
            modifier
                .fillMaxWidth()
                .aspectRatio(aspect)
                .clipToBounds()
                .onSizeChanged { canvas = Size(it.width.toFloat(), it.height.toFloat()) }
                .transformable(transform, canPan = { zoom > 1.01f })
                .let { m ->
                    if (onTap == null) m else m.pointerInput(area, zoom, pan) {
                        detectTapGestures { tap ->
                            val frame = frame(Size(size.width.toFloat(), size.height.toFloat()), area, zoom, pan)
                            val px = (tap.x - frame.offset.x) / frame.scale
                            val py = (tap.y - frame.offset.y) / frame.scale
                            onTap(Coordinate(latitude = -py.toDouble(), longitude = px.toDouble() / COS))
                        }
                    }
                }
                .semantics { contentDescription = description },
        ) {
            drawRect(sea)
            val frame = frame(size, area, zoom, pan)
            withTransform({
                translate(frame.offset.x, frame.offset.y)
                scale(frame.scale, frame.scale, Offset.Zero)
            }) {
                val hairline = 1f / frame.scale
                paths.forEach { (code, path) ->
                    drawPath(path, fills[code]?.let { Intensities.color(it) } ?: land)
                }
                paths.values.forEach { drawPath(it, border, style = Stroke(width = hairline)) }
                for (wave in waves) {
                    wave.sRadiusKm?.let { circle(wave.epicentre, it, Color(0x40D32F2F), Color(0xFFD32F2F), 2.5f * hairline, filled = true) }
                    wave.pRadiusKm?.let { circle(wave.epicentre, it, Color.Transparent, Color(0xFF1E64C8), 2f * hairline, filled = false) }
                }
            }
            val toScreen = { c: Coordinate -> Offset(frame.offset.x + x(c.longitude) * frame.scale, frame.offset.y + y(c.latitude) * frame.scale) }
            place?.let {
                val p = toScreen(it)
                drawCircle(Color.White, 9.dp.toPx(), p)
                drawCircle(placeColor, 7.dp.toPx(), p)
                drawCircle(Color.White, 3.dp.toPx(), p)
            }
            epicentre?.let { cross(toScreen(it), marker) }
        }
    }

    private data class Frame(val scale: Float, val offset: Offset)

    /** The scale and offset that fit `area` in `size`, then zoomed about the centre and moved. */
    private fun frame(size: Size, area: Area, zoom: Float, pan: Offset): Frame {
        val width = x(area.east) - x(area.west)
        val height = y(area.south) - y(area.north)
        val fit = min(size.width / width, size.height / height)
        val scale = fit * zoom
        val centreX = (x(area.west) + x(area.east)) / 2
        val centreY = (y(area.north) + y(area.south)) / 2
        return Frame(scale, Offset(size.width / 2 - centreX * scale + pan.x, size.height / 2 - centreY * scale + pan.y))
    }

    private fun DrawScope.circle(centre: Coordinate, radiusKm: Double, fill: Color, stroke: Color, width: Float, filled: Boolean) {
        val ry = (radiusKm / 111.2).toFloat()
        val rx = ry * COS / max(0.2f, cos(centre.latitude * PI / 180).toFloat())
        val topLeft = Offset(x(centre.longitude) - rx, y(centre.latitude) - ry)
        val size = Size(rx * 2, ry * 2)
        if (filled) drawOval(fill, topLeft, size)
        drawOval(stroke, topLeft, size, style = Stroke(width = width))
    }

    /** The epicentre: a cross with a light outline, legible on any colour. */
    private fun DrawScope.cross(at: Offset, color: Color) {
        val arm = 10.dp.toPx()
        translate(at.x, at.y) {
            listOf(Color.White to 7.dp.toPx(), color to 3.5.dp.toPx()).forEach { (c, w) ->
                drawLine(c, Offset(-arm, -arm), Offset(arm, arm), w, StrokeCap.Round)
                drawLine(c, Offset(-arm, arm), Offset(arm, -arm), w, StrokeCap.Round)
            }
        }
    }

    /** The area to show for an earthquake: around its epicentre and the user's place, or Japan. */
    fun areaFor(epicentre: Coordinate?, place: Coordinate?, fills: Set<Int>, shapes: List<MapShape>): Area {
        val points = buildList {
            epicentre?.let { add(it) }
            place?.let { add(it) }
            shapes.filter { it.prefecture in fills }.forEach { s -> s.rings.forEach { r -> r.forEach { add(Coordinate(it[1], it[0])) } } }
        }
        if (points.isEmpty()) return JAPAN
        val south = points.minOf { it.latitude }
        val north = points.maxOf { it.latitude }
        val west = points.minOf { it.longitude }
        val east = points.maxOf { it.longitude }
        // At least 300 km across, with a margin, so that one point is shown in its region.
        val centre = Coordinate((south + north) / 2, (west + east) / 2)
        val span = max(max(north - south, (east - west) * cos(centre.latitude * PI / 180)) * 111.2 / 2 * 1.25, 150.0)
        return Area.around(centre, span)
    }
}
