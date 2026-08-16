//! Shared `chart`/`chart_series` IR translation (ADR 0016:
//! `docs/adr/0016-chart-ir-shape.md`), used by both the XLSX and PPTX
//! reader/writer — both share the exact same DrawingML `<c:chartSpace>`
//! chart-part schema (ECMA-376 §21.2), the former at `xl/charts/chartN.xml`
//! and the latter at `ppt/charts/chartN.xml`, parsed by the same
//! `ooxml_sml::parse_chart_xml` hand-rolled walker (see that function's doc
//! comment for why chart-XML parsing lives in `ooxml-sml` rather than
//! `ooxml-pml`, and why this crate doesn't duplicate it).

use ooxml_sml::{Chart, ChartSeries, ChartType};
use rescribe_core::PropValue;
use rescribe_std::{Node, node, prop};

/// Convert an OOXML chart (ADR 0016) into a `chart` node with `chart_series`
/// children. The raw chart-part XML is always attached via
/// `prop::OOXML_CHART_XML` (ADR 0016 Decision 4), unconditionally — not only
/// when the semantic fields below don't cover something.
pub(crate) fn convert_chart(chart: &Chart) -> Node {
    let mut node = Node::new(node::CHART)
        .prop(prop::CHART_TYPE, chart_type_str(chart.chart_type))
        .prop(prop::CHART_LEGEND, chart.legend)
        .prop(prop::CHART_HAS_CATEGORY_AXIS, chart.has_category_axis)
        .prop(prop::CHART_HAS_VALUE_AXIS, chart.has_value_axis)
        .prop(prop::OOXML_CHART_XML, chart.raw_xml.clone());
    if let Some(title) = &chart.title {
        node = node.prop(prop::TITLE, title.clone());
    }
    if chart.legend
        && let Some(pos) = &chart.legend_position
    {
        node = node.prop(prop::CHART_LEGEND_POSITION, pos.clone());
    }
    node.children(chart.series.iter().map(convert_chart_series))
}

fn convert_chart_series(series: &ChartSeries) -> Node {
    let mut node = Node::new(node::CHART_SERIES);
    if let Some(name) = series.name() {
        node = node.prop(prop::TITLE, name.to_string());
    }
    if !series.values().is_empty() {
        node = node.prop(
            prop::CHART_VALUES,
            PropValue::List(
                series
                    .values()
                    .iter()
                    .map(|v| PropValue::Float(*v))
                    .collect(),
            ),
        );
    }
    if let Some(value_ref) = series.value_ref() {
        node = node.prop(prop::CHART_VALUES_REF, value_ref.to_string());
    }
    if !series.categories().is_empty() {
        node = node.prop(
            prop::CHART_CATEGORIES,
            PropValue::List(
                series
                    .categories()
                    .iter()
                    .map(|c| PropValue::String(c.clone()))
                    .collect(),
            ),
        );
    }
    if let Some(cat_ref) = series.category_ref() {
        node = node.prop(prop::CHART_CATEGORIES_REF, cat_ref.to_string());
    }
    node
}

/// Map `ooxml_sml`'s chart-type enum to the ADR 0016 Decision 3 open-string
/// vocabulary. Names are the union of OOXML's plot-type element names
/// (lowercased, without the `Chart` suffix); `column` is reserved for a
/// future OOXML `barChart`/`barDir` distinction (bar vs. column) that
/// `ooxml-sml`'s current hand-rolled parser doesn't yet distinguish (both map
/// to `ChartType::Bar`/`Bar3D`).
fn chart_type_str(t: ChartType) -> &'static str {
    match t {
        ChartType::Bar => "bar",
        ChartType::Column => "column",
        ChartType::Line => "line",
        ChartType::Pie => "pie",
        ChartType::Area => "area",
        ChartType::Scatter => "scatter",
        ChartType::Doughnut => "doughnut",
        ChartType::Radar => "radar",
        ChartType::Surface => "surface",
        ChartType::Bubble => "bubble",
        ChartType::Stock => "stock",
        ChartType::Unknown => "unknown",
    }
}

/// Map the ADR 0016 Decision 3 open-string `chart:type` vocabulary back to
/// an OOXML plot-type element local name, the inverse of [`chart_type_str`].
/// Unrecognized/absent types default to `lineChart`, matching
/// `ooxml_sml::workbook::ChartType`'s own `Default` (`Line`).
fn chart_type_xml_tag(t: Option<&str>) -> &'static str {
    match t {
        Some("bar") | Some("column") => "barChart",
        Some("pie") => "pieChart",
        Some("area") => "areaChart",
        Some("scatter") => "scatterChart",
        Some("doughnut") => "doughnutChart",
        Some("radar") => "radarChart",
        Some("surface") => "surfaceChart",
        Some("bubble") => "bubbleChart",
        Some("stock") => "stockChart",
        _ => "lineChart",
    }
}

/// Minimal XML-escape for text content (not attribute values — no attribute
/// text is emitted by [`build_minimal_chart_xml`] besides fixed constants).
fn xml_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Best-effort fallback: build a minimal, valid `<c:chartSpace>` document
/// (ECMA-376 §21.2) from a `chart` node's *semantic* fields only. Used only
/// when a `chart` node has no `prop::OOXML_CHART_XML` raw fallback at all —
/// the overwhelmingly common case (an OOXML-sourced chart) re-emits the raw
/// XML verbatim instead and never reaches this function. Covers the same v1
/// semantic-core scope as the reader (ADR 0016 Decision 4): chart type,
/// title, one series per `chart_series` child with literal-or-referenced
/// values/categories, legend presence/position, and category/value axis
/// presence. Anything beyond that (3D variants, combo charts, trendlines,
/// error bars, per-point styling) is not reconstructable from the IR in v1
/// and is simply absent from the generated XML — a real (documented)
/// content gap for chart nodes with no raw fallback, not a bug in this
/// function.
pub(crate) fn build_minimal_chart_xml(node: &Node) -> String {
    let chart_type = node.props.get_str(prop::CHART_TYPE);
    let tag = chart_type_xml_tag(chart_type);

    let mut series_xml = String::new();
    for (i, series) in node
        .children
        .iter()
        .filter(|c| c.kind.as_str() == node::CHART_SERIES)
        .enumerate()
    {
        series_xml.push_str(&format!(
            "<c:ser><c:idx val=\"{i}\"/><c:order val=\"{i}\"/>"
        ));
        if let Some(title) = series.props.get_str(prop::TITLE) {
            series_xml.push_str(&format!(
                "<c:tx><c:strRef><c:f></c:f><c:strCache><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>{}</c:v></c:pt></c:strCache></c:strRef></c:tx>",
                xml_escape_text(title)
            ));
        }
        // Categories: `c:cat`, wrapped in `c:strRef` regardless of whether
        // the series is reference-backed — matching the reader's own
        // shape (`ooxml-sml::workbook::parse_chart`), which tolerates an
        // absent/empty `<c:f>` for the literal (non-referenced) case.
        let categories = series.props.get(prop::CHART_CATEGORIES);
        if let Some(PropValue::List(cats)) = categories {
            let cat_ref = series.props.get_str(prop::CHART_CATEGORIES_REF);
            let mut pts = String::new();
            for (pi, c) in cats.iter().enumerate() {
                let text = match c {
                    PropValue::String(s) => s.clone(),
                    other => format!("{:?}", other),
                };
                pts.push_str(&format!(
                    "<c:pt idx=\"{pi}\"><c:v>{}</c:v></c:pt>",
                    xml_escape_text(&text)
                ));
            }
            series_xml.push_str(&format!(
                "<c:cat><c:strRef><c:f>{}</c:f><c:strCache><c:ptCount val=\"{}\"/>{}</c:strCache></c:strRef></c:cat>",
                xml_escape_text(cat_ref.unwrap_or_default()),
                cats.len(),
                pts
            ));
        }
        // Values: `c:val`, wrapped in `c:numRef`, same reference-or-literal
        // tolerance as categories above.
        let values = series.props.get(prop::CHART_VALUES);
        if let Some(PropValue::List(vals)) = values {
            let val_ref = series.props.get_str(prop::CHART_VALUES_REF);
            let mut pts = String::new();
            for (pi, v) in vals.iter().enumerate() {
                let num = match v {
                    PropValue::Float(f) => *f,
                    PropValue::Int(n) => *n as f64,
                    _ => 0.0,
                };
                pts.push_str(&format!("<c:pt idx=\"{pi}\"><c:v>{num}</c:v></c:pt>"));
            }
            series_xml.push_str(&format!(
                "<c:val><c:numRef><c:f>{}</c:f><c:numCache><c:ptCount val=\"{}\"/>{}</c:numCache></c:numRef></c:val>",
                xml_escape_text(val_ref.unwrap_or_default()),
                vals.len(),
                pts
            ));
        }
        series_xml.push_str("</c:ser>");
    }

    let title_xml = match node.props.get_str(prop::TITLE) {
        Some(title) => format!(
            "<c:title><c:tx><c:rich><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title>",
            xml_escape_text(title)
        ),
        None => String::new(),
    };

    let mut axes_xml = String::new();
    if node
        .props
        .get_bool(prop::CHART_HAS_CATEGORY_AXIS)
        .unwrap_or(false)
    {
        axes_xml.push_str("<c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"b\"/><c:crossAx val=\"2\"/></c:catAx>");
    }
    if node
        .props
        .get_bool(prop::CHART_HAS_VALUE_AXIS)
        .unwrap_or(false)
    {
        axes_xml.push_str("<c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"l\"/><c:crossAx val=\"1\"/></c:valAx>");
    }

    let legend_xml = if node.props.get_bool(prop::CHART_LEGEND).unwrap_or(false) {
        let pos = node
            .props
            .get_str(prop::CHART_LEGEND_POSITION)
            .map(|p| match p {
                "bottom" => "b",
                "left" => "l",
                "right" => "r",
                "top" => "t",
                "top-right" => "tr",
                other => other,
            })
            .unwrap_or("r");
        format!("<c:legend><c:legendPos val=\"{pos}\"/></c:legend>")
    } else {
        String::new()
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<c:chart>{title_xml}<c:plotArea><c:layout/><c:{tag}>{series_xml}</c:{tag}>{axes_xml}</c:plotArea>{legend_xml}\
<c:plotVisOnly val=\"1\"/></c:chart></c:chartSpace>"
    )
}
