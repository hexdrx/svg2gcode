use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use g_code::{
    emit::{format_gcode_fmt, format_gcode_io, FormatOptions},
    parse::snippet_parser,
};
use js_sys::Date;
use log::Level;
use roxmltree::{Document, ParsingOptions};
use svg2gcode::{svg2program, ConversionOptions, Machine};
use yew::prelude::*;

mod forms;
mod state;
mod ui;
mod util;

use forms::*;
use state::*;
use ui::*;
use util::*;
use yewdux::{prelude::use_store, use_dispatch, YewduxRoot};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

#[function_component(App)]
fn app() -> Html {
    let generating = use_state_eq(|| false);
    let generating_setter = generating.setter();

    let form_dispatch = use_dispatch::<FormState>();
    let (app_store, app_dispatch) = use_store::<AppState>();

    // TODO: come up with a less awkward way to do this.
    // Having separate stores is somewhat of an anti-pattern in Redux,
    // but there's no easy way to do hydration after the app state is
    // restored from local storage.
    let upgraded_settings_and_hydrated_form = use_state(|| false);
    if !*upgraded_settings_and_hydrated_form {
        app_dispatch.reduce_mut(|app| {
            if app.settings.try_upgrade().is_err() {
                unreachable!("No breaking upgrades yet!")
            }
            let hydrated_form_state = FormState::from(&app_store.settings);
            form_dispatch.reduce_mut(|state| *state = hydrated_form_state);
        });
        upgraded_settings_and_hydrated_form.set(true);
    }

    let generate_disabled = *generating || app_store.svgs.is_empty();
    let generate_onclick = {
        let app_store = app_store.clone();
        Callback::from(move |_| {
            generating_setter.set(true);
            let mut zip = ZipWriter::new(Cursor::new(vec![]));
            let opts = FileOptions::default().compression_method(CompressionMethod::Stored);

            if app_store.svgs.len() > 1 {
                zip.add_directory("svg2gcode_output", opts).unwrap();
            }

            for svg in app_store.svgs.iter() {
                let options = ConversionOptions {
                    dimensions: svg.dimensions,
                };

                // Apply scale by adjusting DPI (higher DPI = smaller output, so divide by scale)
                let mut scaled_conversion_config = app_store.settings.conversion.clone();
                scaled_conversion_config.dpi = scaled_conversion_config.dpi / svg.scale;

                // Apply offset
                scaled_conversion_config.origin = [
                    Some(svg.offset[0]),
                    Some(svg.offset[1]),
                ];

                let machine = Machine::new(
                    app_store.settings.machine.supported_functionality.clone(),
                    app_store
                        .settings
                        .machine
                        .tool_on_sequence
                        .as_deref()
                        .map(snippet_parser)
                        .transpose()
                        .unwrap(),
                    app_store
                        .settings
                        .machine
                        .tool_off_sequence
                        .as_deref()
                        .map(snippet_parser)
                        .transpose()
                        .unwrap(),
                    app_store
                        .settings
                        .machine
                        .begin_sequence
                        .as_deref()
                        .map(snippet_parser)
                        .transpose()
                        .unwrap(),
                    app_store
                        .settings
                        .machine
                        .end_sequence
                        .as_deref()
                        .map(snippet_parser)
                        .transpose()
                        .unwrap(),
                );
                let document = Document::parse_with_options(
                    svg.content.as_str(),
                    ParsingOptions {
                        allow_dtd: true,
                        ..Default::default()
                    },
                )
                .unwrap();

                let program =
                    svg2program(&document, &scaled_conversion_config, options, machine);

                let filepath = if app_store.svgs.len() > 1 {
                    PathBuf::from("svg2gcode_output")
                        .join(Path::new(svg.filename.as_str()).with_extension("gcode"))
                } else {
                    Path::new(svg.filename.as_str()).with_extension("gcode")
                };

                match app_store.svgs.len() {
                    0 => unreachable!(),
                    1 => {
                        let gcode = {
                            let mut acc = String::new();
                            format_gcode_fmt(
                                &program,
                                FormatOptions {
                                    checksums: app_store.settings.postprocess.checksums,
                                    line_numbers: app_store.settings.postprocess.line_numbers,
                                    newline_before_comment: app_store
                                        .settings
                                        .postprocess
                                        .newline_before_comment,
                                    ..Default::default()
                                },
                                &mut acc,
                            )
                            .unwrap();
                            acc
                        };
                        prompt_download(filepath, gcode.as_bytes());
                    }
                    _multiple => {
                        zip.start_file(filepath.to_string_lossy(), opts).unwrap();

                        format_gcode_io(
                            &program,
                            FormatOptions {
                                checksums: app_store.settings.postprocess.checksums,
                                line_numbers: app_store.settings.postprocess.line_numbers,
                                newline_before_comment: app_store
                                    .settings
                                    .postprocess
                                    .newline_before_comment,
                                ..Default::default()
                            },
                            &mut zip,
                        )
                        .unwrap();
                    }
                }
            }

            if app_store.svgs.len() > 1 {
                zip.set_comment(format!(
                    "Created with svg2gcode: https://sameer.github.io/svg2gcode/\n{}",
                    env!("CARGO_PKG_DESCRIPTION")
                ));
                let output = zip.finish().unwrap();
                let date = Date::new_0().to_iso_string();
                prompt_download(
                    format!("svg2gcode_bulk_download_{date}.zip"),
                    output.get_ref(),
                );
            }

            generating_setter.set(false);
        })
    };

    html! {
        <div class="container">
            <div class={classes!("column")}>
                <h1>
                    { "svg2gcode" }
                </h1>
                <p>
                    { env!("CARGO_PKG_DESCRIPTION") }
                </p>
                <SvgForm/>
                <ButtonGroup>
                    <Button
                        title="Generate G-Code"
                        style={ButtonStyle::Primary}
                        loading={*generating}
                        icon={
                            html_nested! (
                                <Icon name={IconName::Download} />
                            )
                        }
                        disabled={generate_disabled}
                        onclick={generate_onclick}
                    />
                    <HyperlinkButton
                        title="Settings"
                        style={ButtonStyle::Default}
                        icon={IconName::Edit}
                        href="#settings"
                    />
                </ButtonGroup>
                <div class={classes!("card-container", "columns")}>
                    {
                        for app_store.svgs.iter().enumerate().map(|(i, svg)| {
                            let svg_content = svg.content.clone();
                            let svg_scale = svg.scale;
                            let svg_filename = svg.filename.clone();
                            let svg_dimensions = svg.dimensions;
                            let svg_offset = svg.offset;

                            let remove_svg_onclick = app_dispatch.reduce_mut_callback(move |app| {
                                app.svgs.remove(i);
                            });

                            let scale_oninput = app_dispatch.reduce_mut_callback_with(move |app, event: InputEvent| {
                                let value = event.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                if let Ok(scale) = value.parse::<f64>() {
                                    if scale > 0.0 {
                                        app.svgs[i].scale = scale;
                                    }
                                }
                            });

                            let on_offset_change = app_dispatch.reduce_mut_callback_with(move |app, offset: [f64; 2]| {
                                app.svgs[i].offset = offset;
                            });

                            let body = html!{
                                <div>
                                    <SvgPreview
                                        svg_content={svg_content.clone()}
                                        scale={svg_scale}
                                        filename={svg_filename.clone()}
                                        dimensions={svg_dimensions}
                                        offset={svg_offset}
                                        on_offset_change={on_offset_change}
                                    />
                                    <div class="form-group" style="margin-top: 10px;">
                                        <label class="form-label">{"Scale:"}</label>
                                        <input
                                            type="number"
                                            class="form-input"
                                            step="0.1"
                                            min="0.1"
                                            value={svg_scale.to_string()}
                                            oninput={scale_oninput}
                                            style="width: 100%;"
                                        />
                                    </div>
                                </div>
                            };

                            let footer = html!{
                                <Button
                                    title="Remove"
                                    style={ButtonStyle::Primary}
                                    icon={
                                        html_nested!(
                                            <Icon name={IconName::Delete} />
                                        )
                                    }
                                    onclick={remove_svg_onclick}
                                />
                            };
                            html!{
                                <div class={classes!("column", "col-6", "col-xs-12")}>
                                    <Card
                                        title={svg.filename.clone()}
                                        body={body}
                                        footer={footer}
                                    />
                                </div>
                            }
                        })
                    }
                </div>
                <SettingsForm/>
                <ImportExportModal/>
            </div>
            <div class={classes!("text-right", "column")}>
                <p>
                    { "See the project " }
                    <a href={env!("CARGO_PKG_REPOSITORY")}>
                        { "on GitHub" }
                    </a>
                    {" for support" }
                </p>
            </div>
        </div>
    }
}

#[function_component(AppContainer)]
fn app_container() -> Html {
    html! {
        <YewduxRoot>
            <App/>
        </YewduxRoot>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(Level::Info));
    yew::Renderer::<AppContainer>::new().render();
}
