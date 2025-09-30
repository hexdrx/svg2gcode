use std::str::FromStr;
use base64::Engine;
use roxmltree::Document;
use svgtypes::Length;
use yew::prelude::*;
use yewdux::functional::use_store_value;
use crate::state::AppState;

#[derive(Properties, PartialEq, Clone)]
pub struct PreviewProps {
    pub svg_content: String,
    pub scale: f64,
    pub filename: String,
    pub dimensions: [Option<Length>; 2],
}

// Parse SVG size from viewBox or width/height attributes
fn parse_svg_dimensions(svg_content: &str, override_dimensions: [Option<Length>; 2]) -> Option<(f64, f64)> {
    let doc = Document::parse(svg_content).ok()?;
    let root = doc.root_element();

    // Check for dimension overrides first
    let width_mm = if let Some(Length { number, unit, .. }) = override_dimensions[0] {
        // Convert to mm based on unit
        match unit {
            svgtypes::LengthUnit::Mm => Some(number),
            svgtypes::LengthUnit::Cm => Some(number * 10.0),
            svgtypes::LengthUnit::In => Some(number * 25.4),
            svgtypes::LengthUnit::Px => Some(number * 25.4 / 96.0), // Assuming 96 DPI
            svgtypes::LengthUnit::Pt => Some(number * 25.4 / 72.0),
            svgtypes::LengthUnit::Pc => Some(number * 25.4 / 6.0),
            _ => None,
        }
    } else {
        None
    };

    let height_mm = if let Some(Length { number, unit, .. }) = override_dimensions[1] {
        match unit {
            svgtypes::LengthUnit::Mm => Some(number),
            svgtypes::LengthUnit::Cm => Some(number * 10.0),
            svgtypes::LengthUnit::In => Some(number * 25.4),
            svgtypes::LengthUnit::Px => Some(number * 25.4 / 96.0),
            svgtypes::LengthUnit::Pt => Some(number * 25.4 / 72.0),
            svgtypes::LengthUnit::Pc => Some(number * 25.4 / 6.0),
            _ => None,
        }
    } else {
        None
    };

    // If overrides exist, use them
    if let (Some(w), Some(h)) = (width_mm, height_mm) {
        return Some((w, h));
    }

    // Try to get width/height from attributes
    let width = root.attribute("width")
        .and_then(|w| Length::from_str(w).ok())
        .or(override_dimensions[0]);

    let height = root.attribute("height")
        .and_then(|h| Length::from_str(h).ok())
        .or(override_dimensions[1]);

    if let (Some(w), Some(h)) = (width, height) {
        // Convert to mm (assuming pixels with 96 DPI if no unit)
        let w_mm = match w.unit {
            svgtypes::LengthUnit::Mm => w.number,
            svgtypes::LengthUnit::Cm => w.number * 10.0,
            svgtypes::LengthUnit::In => w.number * 25.4,
            svgtypes::LengthUnit::Px | svgtypes::LengthUnit::None => w.number * 25.4 / 96.0,
            svgtypes::LengthUnit::Pt => w.number * 25.4 / 72.0,
            svgtypes::LengthUnit::Pc => w.number * 25.4 / 6.0,
            _ => w.number * 25.4 / 96.0,
        };

        let h_mm = match h.unit {
            svgtypes::LengthUnit::Mm => h.number,
            svgtypes::LengthUnit::Cm => h.number * 10.0,
            svgtypes::LengthUnit::In => h.number * 25.4,
            svgtypes::LengthUnit::Px | svgtypes::LengthUnit::None => h.number * 25.4 / 96.0,
            svgtypes::LengthUnit::Pt => h.number * 25.4 / 72.0,
            svgtypes::LengthUnit::Pc => h.number * 25.4 / 6.0,
            _ => h.number * 25.4 / 96.0,
        };

        return Some((w_mm, h_mm));
    }

    // Try to parse viewBox
    if let Some(viewbox_str) = root.attribute("viewBox") {
        let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
        if parts.len() == 4 {
            if let (Ok(_x), Ok(_y), Ok(w), Ok(h)) = (
                parts[0].parse::<f64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
                parts[3].parse::<f64>(),
            ) {
                // ViewBox units are typically pixels, convert to mm at 96 DPI
                return Some((w * 25.4 / 96.0, h * 25.4 / 96.0));
            }
        }
    }

    None
}

#[function_component(SvgPreview)]
pub fn svg_preview(props: &PreviewProps) -> Html {
    let app_state = use_store_value::<AppState>();
    let bed_width = app_state.settings.conversion.bed_size[0];
    let bed_height = app_state.settings.conversion.bed_size[1];

    // Parse SVG dimensions in mm
    let svg_dimensions = parse_svg_dimensions(&props.svg_content, props.dimensions);

    // Calculate grid lines (10mm spacing)
    let grid_spacing = 10.0;
    let num_vertical_lines = (bed_width / grid_spacing) as usize;
    let num_horizontal_lines = (bed_height / grid_spacing) as usize;

    // Create SVG with grid
    let view_box = format!("0 0 {} {}", bed_width, bed_height);

    // Encode the original SVG as base64 for display
    let svg_base64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(props.svg_content.as_bytes());

    // Calculate scaled dimensions
    let (scaled_width, scaled_height, dimensions_info) = if let Some((w_mm, h_mm)) = svg_dimensions {
        let scaled_w = w_mm * props.scale;
        let scaled_h = h_mm * props.scale;
        (scaled_w, scaled_h, format!("{:.1}×{:.1} mm", scaled_w, scaled_h))
    } else {
        (0.0, 0.0, "Unknown size".to_string())
    };

    // Check if SVG fits on bed
    let fits_on_bed = scaled_width <= bed_width && scaled_height <= bed_height;
    let warning_color = if fits_on_bed { "#4caf50" } else { "#f44336" };

    html! {
        <div class="svg-preview-container" style="position: relative; width: 100%; aspect-ratio: 1;">
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox={view_box.clone()}
                style="width: 100%; height: 100%; border: 1px solid #ccc; background: white;"
            >
                // Grid lines
                <g class="grid" stroke="#e0e0e0" stroke-width="0.5">
                    {
                        for (0..=num_vertical_lines).map(|i| {
                            let x = i as f64 * grid_spacing;
                            html! {
                                <line
                                    x1={x.to_string()}
                                    y1="0"
                                    x2={x.to_string()}
                                    y2={bed_height.to_string()}
                                />
                            }
                        })
                    }
                    {
                        for (0..=num_horizontal_lines).map(|i| {
                            let y = i as f64 * grid_spacing;
                            html! {
                                <line
                                    x1="0"
                                    y1={y.to_string()}
                                    x2={bed_width.to_string()}
                                    y2={y.to_string()}
                                />
                            }
                        })
                    }
                </g>

                // Bed border
                <rect
                    x="0"
                    y="0"
                    width={bed_width.to_string()}
                    height={bed_height.to_string()}
                    fill="none"
                    stroke="#333"
                    stroke-width="1"
                />

                // SVG content as image with proper sizing
                if svg_dimensions.is_some() {
                    <image
                        href={format!("data:image/svg+xml;base64,{}", svg_base64)}
                        x="0"
                        y="0"
                        width={scaled_width.to_string()}
                        height={scaled_height.to_string()}
                        preserveAspectRatio="xMinYMin meet"
                    />

                    // Draw outline box around SVG area
                    <rect
                        x="0"
                        y="0"
                        width={scaled_width.to_string()}
                        height={scaled_height.to_string()}
                        fill="none"
                        stroke={warning_color}
                        stroke-width="1"
                        stroke-dasharray="5,5"
                    />
                }
            </svg>
            <div style={format!("position: absolute; bottom: 5px; right: 5px; font-size: 10px; background: rgba(255,255,255,0.9); padding: 3px 6px; border-left: 3px solid {};", warning_color)}>
                <div>{format!("Bed: {}×{} mm", bed_width, bed_height)}</div>
                <div><strong>{format!("SVG: {}", dimensions_info)}</strong></div>
                <div>{format!("Scale: {:.2}x", props.scale)}</div>
                if !fits_on_bed && svg_dimensions.is_some() {
                    <div style="color: #f44336;"><strong>{"⚠ Too large for bed!"}</strong></div>
                }
            </div>
        </div>
    }
}