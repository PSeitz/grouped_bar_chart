use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader};

use svg::node::element::{self, Group as SVGGroup};
use svg::node::element::{Line, Rectangle};
use svg::{Document, Node};

struct BenchData {
    bench_name: String,
    group_name: String,
    variant: String,
    num_bytes: u32,
    gbs: f64,
}
impl Debug for BenchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchData")
            .field("bench_name", &self.bench_name)
            .field("group_name", &self.group_name)
            .field("variant", &self.variant)
            .field("num_bytes", &self.num_bytes)
            .field("gbs", &self.gbs)
            .finish()
    }
}

fn load_data() -> BTreeMap<String, Vec<BenchData>> {
    let mut groups = BTreeMap::new();
    //let mut data = Vec::new();
    let file = File::open("./data.json").unwrap();
    for line in BufReader::new(file).lines() {
        let line = line.unwrap();
        let val: serde_json::Value = serde_json::from_str(&line).unwrap();
        let reason = val["reason"].as_str().unwrap();
        if reason != "benchmark-complete" {
            continue;
        }
        let name = val["id"].as_str().unwrap().to_string();
        let components: Vec<String> = name.split("/").map(|el| el.to_string()).collect();
        let bench_name = components[0].to_string();
        let variant = components[1].to_string();
        let num_bytes = components[2].to_string();
        let duration_ns = val["typical"]["estimate"].as_f64().unwrap();

        let num_bytes: u32 = num_bytes.parse().unwrap();

        let group_name = format!("{}/{}", bench_name, num_bytes);

        let gbs = num_bytes as f64 / duration_ns;
        //data.push((bench_name, group_name, variant, num_bytes, gbs));

        let blub: &mut Vec<_> = groups.entry(group_name.to_string()).or_default();

        blub.push(BenchData {
            bench_name,
            group_name,
            variant,
            num_bytes,
            gbs,
        });
    }
    dbg!(&groups);
    groups
}

fn main() {
    let chart_title = std::env::args().skip(1).next().unwrap();
    let name_to_benches = load_data();
    let variants = name_to_benches
        .iter()
        .flat_map(|group| group.1.iter())
        .map(|b| b.variant.to_string())
        .collect::<BTreeSet<_>>();

    let mut colors = vec![
        "#37123C".to_string(),
        "#71677C".to_string(),
        "#A99F96".to_string(),
        "#DDA77B".to_string(),
    ];

    let variant_to_color: BTreeMap<String, String> = variants
        .iter()
        .map(|variant| (variant.to_string(), colors.pop().unwrap().to_string()))
        .collect();

    let mut groups = vec![];

    for (_name, group) in name_to_benches.iter() {
        let values_and_color = group
            .iter()
            .map(|run| {
                (
                    run.gbs as f32,
                    variant_to_color.get(&run.variant).unwrap().to_string(),
                )
            })
            .collect();
        let gruppe = Group {
            label: num_bytes_to_name(group[0].num_bytes),
            values_and_color,
        };
        groups.push(gruppe);
    }

    let opt = GroupBarOptions {
        total_width: 800.0,
        total_height: 600.0,
        chart_area_to_border_padding: 10.0,
        group_padding: 20.0,
        bar_padding: 3.0,
        print_delta: true,
    };

    let mut document = element::Group::new();
    document = document.set("font-family", "Roboto-Regular,Roboto, sans-serif");

    let document = render_grouped_bar_chart(&chart_title, document, opt, &groups, variant_to_color);

    svg::save("image.svg", &Document::new().add(document)).unwrap();
}

fn num_bytes_to_name(num_bytes: u32) -> String {
    match num_bytes {
        725 => "725b Text".to_string(),
        66675 => "66K JSON".to_string(),
        64723 => "65K Text".to_string(),
        9991663 => "10Mb Dickens".to_string(),
        34308 => "34K Text".to_string(),
        _ => num_bytes.to_string(),
    }
}

const X_AXIS_SPACE: f32 = 80.0;
#[derive(Debug)]
struct GroupBarOptions {
    total_width: f32,
    total_height: f32,
    /// chart padding from border
    chart_area_to_border_padding: f32,
    /// padding between groups
    group_padding: f32,
    /// padding between bars inside group
    bar_padding: f32,
    print_delta: bool,
}
impl GroupBarOptions {
    fn get_available_graph_width(&self) -> f32 {
        let y_axis_space = 80.0;
        self.total_width - y_axis_space - self.chart_area_to_border_padding * 2.0
    }
    fn get_available_graph_height(&self) -> f32 {
        let x_axis_space = X_AXIS_SPACE;
        self.total_height - x_axis_space - self.chart_area_to_border_padding * 2.0
    }
}

#[derive(Debug)]
struct Group {
    label: String,
    values_and_color: Vec<(f32, String)>,
}

fn compute_y_for_value(options: &GroupBarOptions, val: f32, max_value: f32) -> f32 {
    let max_height = options.get_available_graph_height();
    let bar_start = max_height + options.chart_area_to_border_padding;
    let height = max_height * (val / max_value);
    bar_start - height
}

fn draw_group(
    doc: SVGGroup,
    options: &GroupBarOptions,
    groups: &Group,
    group_start_x: f32,
    bar_width: f32,
    group_width: f32,
    bar_padding: f32,
    max_value: f32,
) -> SVGGroup {
    let max_height = options.get_available_graph_height();
    let bar_start = max_height + options.chart_area_to_border_padding;
    let mut group = doc;
    let mut bar_x = group_start_x;
    for (val, color) in groups.values_and_color.iter() {
        let height = max_height * (val / max_value);
        let y = compute_y_for_value(options, *val, max_value);
        let rect = Rectangle::new()
            .set("x", bar_x)
            .set("y", y)
            .set("width", bar_width)
            .set("height", height)
            .set("fill", color.to_string());

        group = group.add(rect);
        bar_x += (bar_width) + bar_padding;
    }

    let mut node = svg::node::element::Text::new()
        .set("text-anchor", "middle")
        .set("x", group_start_x + bar_width)
        .set("y", bar_start + 20.0);
    node.append(svg::node::Text::new(groups.label.to_string()));
    group = group.add(node);

    if options.print_delta {
        let min = groups
            .values_and_color
            .iter()
            .map(|el| el.0)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let max = groups
            .values_and_color
            .iter()
            .map(|el| el.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let y = compute_y_for_value(options, max, max_value);
        let mut node = svg::node::element::Text::new()
            .set("text-anchor", "middle")
            .set("x", group_start_x + bar_width)
            .set("y", y - 10.0);
        node.append(svg::node::Text::new(get_percent_difference(min, max)));
        group = group.add(node);
    }

    group
}

fn get_percent_difference(min: f32, max: f32) -> String {
    let difference = max - min;
    let percent_difference = (difference / min) * 100.0;
    format!("+{:.2}%", percent_difference)
}

//
fn render_grouped_bar_chart(
    title: &str,
    mut doc: SVGGroup,
    options: GroupBarOptions,
    groups: &[Group],
    variant_to_color: BTreeMap<String, String>,
) -> SVGGroup {
    let max_value: f32 = groups
        .iter()
        .flat_map(|g| &g.values_and_color)
        .map(|el| el.0)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let available_graph_space = options.get_available_graph_width();
    let group_width = available_graph_space / groups.len() as f32;

    let max_num_bars_in_group = groups
        .iter()
        .map(|g| g.values_and_color.len())
        .max()
        .unwrap();
    let bar_width = (group_width / max_num_bars_in_group as f32).min(30.0);

    let mut group_start_x = Vec::new();
    let mut curr_group_x = X_AXIS_SPACE + options.chart_area_to_border_padding;

    doc = draw_y_scale(doc, &options, "Gb/s", curr_group_x, max_value);
    doc = draw_x_scale(doc, &options, "Gb/s", curr_group_x, max_value);

    for group in groups {
        doc = draw_group(
            doc,
            &options,
            group,
            curr_group_x,
            bar_width,
            group_width,
            options.bar_padding,
            max_value,
        );
        group_start_x.push(curr_group_x);
        curr_group_x += group_width;
    }

    // Add legend group
    let mut legend_group = element::Group::new();
    legend_group = draw_legend(legend_group, &options, &variant_to_color);
    legend_group = legend_group.set(
        "transform",
        format!(
            "translate({},{})",
            options.get_available_graph_width() as u32 - 100,
            20
        ),
    );
    doc = doc.add(legend_group);
    doc = doc.set("transform", "translate(0,50)");

    // Add Title
    let mut node = svg::node::element::Text::new()
        .set("text-anchor", "middle")
        .set("font-weight", "bold")
        .set(
            "x",
            options.chart_area_to_border_padding + options.get_available_graph_width() - 70.0,
        )
        .set("y", 0);
    node.append(svg::node::Text::new(title.to_string()));
    doc = doc.add(node);

    doc
}

fn draw_legend(
    mut group: SVGGroup,
    options: &GroupBarOptions,
    variant_to_color: &BTreeMap<String, String>,
) -> SVGGroup {
    let legend_padding = 10;
    let lebend_entry_height = 20;
    let legend_width = 120;
    let legend_height = legend_padding * 2 + variant_to_color.len() * lebend_entry_height;
    let rect = Rectangle::new()
        .set("width", legend_width)
        .set("height", legend_height)
        .set("fill", "#FFFFFF")
        .set("stroke", "#121212");
    group = group.add(rect);
    let mut variant_y = legend_padding + 5;
    for (label, color) in variant_to_color {
        let mut node = svg::node::element::Text::new()
            .set("font-size", 12)
            .set("x", 10)
            .set("y", variant_y + 10);
        node.append(svg::node::Text::new(label.to_string()));
        group = group.add(node);

        let rect = Rectangle::new()
            .set("y", variant_y)
            .set("x", legend_width - 30)
            .set("width", 20)
            .set("height", lebend_entry_height - 10)
            .set("fill", color.to_string());
        group = group.add(rect);
        variant_y += lebend_entry_height;
    }

    group
}

fn draw_y_scale(
    mut group: SVGGroup,
    options: &GroupBarOptions,
    axis_label: &str,
    group_start_x: f32,
    max_value: f32,
) -> SVGGroup {
    let num_markings = 8;

    let axis_x_pos = group_start_x - 5.0;

    let axis = Line::new()
        .set("x1", axis_x_pos)
        .set("y1", options.chart_area_to_border_padding)
        .set("x2", axis_x_pos)
        .set(
            "y2",
            options.chart_area_to_border_padding + options.get_available_graph_height(),
        )
        //.set("width", bar_width)
        .set("stroke", "#000000".to_string());

    // Add ticks
    let ticks = bar_axis_ticks(max_value, num_markings);
    for val in ticks {
        let y = compute_y_for_value(options, val, max_value);
        let tick_line = Line::new()
            .set("x1", axis_x_pos)
            .set("y1", y)
            .set("x2", axis_x_pos - 5.0)
            .set("y2", y)
            .set("stroke", "#000000".to_string());
        group = group.add(tick_line);

        // Add grid
        let tick_line = Line::new()
            .set("x1", axis_x_pos - 5.0)
            .set("y1", y)
            .set(
                "x2",
                options.bar_padding + options.get_available_graph_width(),
            )
            .set("y2", y)
            .set("stroke", "#999999".to_string());
        group = group.add(tick_line);

        let mut node = svg::node::element::Text::new()
            .set("font-size", 12)
            .set("direction", "rtl")
            //.set("text-anchor", "right")
            .set("x", axis_x_pos - 10.0)
            .set("y", y + 4.0);
        node.append(svg::node::Text::new(val.to_string()));
        group = group.add(node);
    }

    let mid_point =
        (options.chart_area_to_border_padding + options.get_available_graph_height()) / 2.0;
    let mut node = svg::node::element::Text::new()
        .set("text-anchor", "middle")
        .set("x", 30)
        .set("y", mid_point);
    node.append(svg::node::Text::new(axis_label.to_string()));
    group = group.add(node);

    group = group.add(axis);

    group
}

fn bar_axis_ticks(max: f32, num_ticks: usize) -> Vec<f32> {
    let step_size = calc_step_size(max as f64, num_ticks as f64) as f32;
    let mut ticks = Vec::with_capacity(num_ticks);
    for i in 0..num_ticks {
        ticks.push(i as f32 * step_size);
    }

    ticks
}

fn calc_step_size(max_val: f64, target_steps: f64) -> f64 {
    // calculate an initial guess at step size
    let temp_step = max_val / target_steps;

    // get the magnitude of the step size
    let mag = f64::floor(f64::ln(temp_step) / std::f64::consts::LN_10);
    let mag_pow = f64::powi(10.0, mag as i32);

    // calculate most significant digit of the new step size
    let mag_msd = f64::round(temp_step / mag_pow + 0.5);

    // promote the MSD to either 1, 2, or 5
    let mag_msd = if mag_msd > 5.0 {
        10.0
    } else if mag_msd > 2.0 {
        5.0
    } else if mag_msd > 1.0 {
        2.0
    } else {
        1.0
    };

    mag_msd * mag_pow
}

fn draw_x_scale(
    mut group: SVGGroup,
    options: &GroupBarOptions,
    axis_label: &str,
    group_start_x: f32,
    max_value: f32,
) -> SVGGroup {
    let num_markings = 4;

    let marking_distance = max_value / 4.0;
    //let marking_vals = (1..=num_markings).map(||{

    let rect = Line::new()
        .set("x1", group_start_x - 5.0)
        .set(
            "y1",
            options.chart_area_to_border_padding + options.get_available_graph_height(),
        )
        .set("x2", group_start_x + options.get_available_graph_width())
        .set(
            "y2",
            options.chart_area_to_border_padding + options.get_available_graph_height(),
        )
        //.set("width", bar_width)
        .set("stroke", "#000000".to_string());

    group = group.add(rect);

    group
}

fn load_convert_data() -> Vec<Group> {
    let name_to_benches = load_data();
    let variants = name_to_benches
        .iter()
        .flat_map(|group| group.1.iter())
        .map(|b| b.variant.to_string())
        .collect::<HashSet<_>>();

    let mut colors = vec![
        "#aa4a44".to_string(),
        "#22aaff".to_string(),
        "#aaaaff".to_string(),
    ];

    let variant_to_color: BTreeMap<String, String> = variants
        .iter()
        .map(|variant| (variant.to_string(), colors.pop().unwrap().to_string()))
        .collect();

    let mut groups = vec![];

    for (_name, group) in name_to_benches.iter() {
        let values_and_color = group
            .iter()
            .map(|run| {
                (
                    run.gbs as f32,
                    variant_to_color.get(&run.variant).unwrap().to_string(),
                )
            })
            .collect();
        let gruppe = Group {
            label: group[0].num_bytes.to_string(),
            values_and_color,
        };
        groups.push(gruppe);
    }
    groups
}

use plotters::prelude::*;
const OUT_FILE_NAME: &'static str = "histogram.png";
fn main2() -> Result<(), Box<dyn std::error::Error>> {
    let data = load_convert_data();
    let data1: Vec<f64> = data
        .iter()
        .map(|group| group.values_and_color[0].0 as f64)
        .collect();
    let data2: Vec<f64> = data
        .iter()
        .map(|group| group.values_and_color[1].0 as f64)
        .collect();
    //let names: Vec<&str> = data[0].values_and_color.iter(|el| el.1).collect();

    //let data1 = vec![0f64, 1.0, 1.0, 1.0, 4.0, 2.0, 5.0, 7.0, 8.0];
    //let data2 = vec![0f64, 2.0, 1.0, 1.0, 2.0, 2.0, 5.0, 3.0, 12.0];

    draw_2hist(
        [data1, data2],
        ["lz4flex", "lz4 ref"],
        "cool",
        ("a", "Gb/s"),
        OUT_FILE_NAME.to_string(),
    )?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    //root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", OUT_FILE_NAME);

    Ok(())
}

pub fn draw_2hist(
    datas: [Vec<f64>; 2],
    datas_name: [&str; 2],
    title: &str,
    axes_desc: (&str, &str),
    path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let n = datas.iter().fold(0f64, |max, l| max.max(l.len() as f64));
    let max_y = datas.iter().fold(0f64, |max, l| {
        max.max(l.iter().fold(f64::NAN, |v_max, &v| v.max(v_max)))
    });

    let root = BitMapBackend::new(&path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("Hack", 44, FontStyle::Bold).into_font())
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        //.build_cartesian_2d(
        //[true, false, true, false, true].into_segmented(),
        //0.0..max_y,
        //)?
        .build_cartesian_2d((3..n as u32).into_segmented(), 0.0..max_y)?
        //.build_cartesian_2d((1..n as u32).into_segmented(), 0.0..max_y)?
        .set_secondary_coord(0.0..n, 0.0..max_y);

    chart
        .configure_mesh()
        .disable_x_mesh()
        .y_desc(axes_desc.1)
        .x_desc(axes_desc.0)
        .axis_desc_style(("Hack", 20))
        .draw()?;

    // creating histograms
    let a = datas[0].iter().zip(0..).map(|(y, x)| {
        plotters::element::Rectangle::new(
            [(x as f64 + 0.1, *y), (x as f64 + 0.5, 0f64)],
            Into::<ShapeStyle>::into(&full_palette::BLUEGREY_A700).filled(),
        )
    });
    // creating histograms
    let b = datas[1].iter().zip(0..).map(|(y, x)| {
        plotters::element::Rectangle::new(
            [(x as f64 + 0.5, *y), (x as f64 + 0.9, 0f64)],
            Into::<ShapeStyle>::into(&full_palette::CYAN_800).filled(),
        )
    });

    chart
        .draw_secondary_series(a)?
        .label(axes_desc.1)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &full_palette::BLUE_A700));
    chart
        .draw_secondary_series(b)?
        .label("lelelel")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &full_palette::CYAN_800));

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .label_font(("Hack", 14).into_font())
        .background_style(&WHITE)
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}
