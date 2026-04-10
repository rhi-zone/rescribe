// Generated from ECMA-376 RELAX NG schema.
// Do not edit manually.

#![allow(nonstandard_style)]

use serde::{Deserialize, Serialize};

/// XML namespace URIs used in this schema.
pub mod ns {
    /// Namespace prefix: anim
    pub const ANIM: &str = "urn:oasis:names:tc:opendocument:xmlns:animation:1.0";
    /// Namespace prefix: chart
    pub const CHART: &str = "urn:oasis:names:tc:opendocument:xmlns:chart:1.0";
    /// Namespace prefix: config
    pub const CONFIG: &str = "urn:oasis:names:tc:opendocument:xmlns:config:1.0";
    /// Namespace prefix: db
    pub const DB: &str = "urn:oasis:names:tc:opendocument:xmlns:database:1.0";
    /// Namespace prefix: dc
    pub const DC: &str = "http://purl.org/dc/elements/1.1/";
    /// Namespace prefix: dr3d
    pub const DR3D: &str = "urn:oasis:names:tc:opendocument:xmlns:dr3d:1.0";
    /// Namespace prefix: draw
    pub const DRAW: &str = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
    /// Namespace prefix: fo
    pub const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
    /// Namespace prefix: form
    pub const FORM: &str = "urn:oasis:names:tc:opendocument:xmlns:form:1.0";
    /// Namespace prefix: grddl
    pub const GRDDL: &str = "http://www.w3.org/2003/g/data-view#";
    /// Namespace prefix: math
    pub const MATH: &str = "http://www.w3.org/1998/Math/MathML";
    /// Namespace prefix: meta
    pub const META: &str = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0";
    /// Namespace prefix: number
    pub const NUMBER: &str = "urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0";
    /// Namespace prefix: office
    pub const OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
    /// Namespace prefix: presentation
    pub const PRESENTATION: &str = "urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";
    /// Namespace prefix: rng
    pub const RNG: &str = "http://relaxng.org/ns/structure/1.0";
    /// Namespace prefix: script
    pub const SCRIPT: &str = "urn:oasis:names:tc:opendocument:xmlns:script:1.0";
    /// Namespace prefix: smil
    pub const SMIL: &str = "urn:oasis:names:tc:opendocument:xmlns:smil-compatible:1.0";
    /// Namespace prefix: style
    pub const STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
    /// Namespace prefix: svg
    pub const SVG: &str = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0";
    /// Namespace prefix: table
    pub const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
    /// Namespace prefix: text
    pub const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
    /// Namespace prefix: xforms
    pub const XFORMS: &str = "http://www.w3.org/2002/xforms";
    /// Namespace prefix: xhtml
    pub const XHTML: &str = "http://www.w3.org/1999/xhtml";
    /// Namespace prefix: xlink
    pub const XLINK: &str = "http://www.w3.org/1999/xlink";
}

pub type CURIE = String;

pub type CURIEs = String;

pub type ID = String;

pub type IDREF = String;

pub type IDREFS = String;

pub type NCName = String;

pub type SafeCURIE = String;

pub type Angle = String;

pub type AnyIRI = String;

pub type AnyURI = String;

pub type Base64Binary = Vec<u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boolean {
    #[serde(rename = "true")]
    True,
    #[serde(rename = "false")]
    False,
}

impl std::fmt::Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
        }
    }
}

impl std::str::FromStr for Boolean {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            _ => Err(format!("unknown Boolean value: {}", s)),
        }
    }
}

pub type BorderWidths = String;

pub type CellAddress = String;

pub type CellRangeAddressList = String;

pub type Character = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartDimension {
    #[serde(rename = "x")]
    X,
    #[serde(rename = "y")]
    Y,
    #[serde(rename = "z")]
    Z,
}

impl std::fmt::Display for ChartDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X => write!(f, "x"),
            Self::Y => write!(f, "y"),
            Self::Z => write!(f, "z"),
        }
    }
}

impl std::str::FromStr for ChartDimension {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            _ => Err(format!("unknown ChartDimension value: {}", s)),
        }
    }
}

pub type ClipShape = String;

pub type Color = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommonRefFormatValues {
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "chapter")]
    Chapter,
    #[serde(rename = "direction")]
    Direction,
    #[serde(rename = "text")]
    Text,
}

impl std::fmt::Display for CommonRefFormatValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Page => write!(f, "page"),
            Self::Chapter => write!(f, "chapter"),
            Self::Direction => write!(f, "direction"),
            Self::Text => write!(f, "text"),
        }
    }
}

impl std::str::FromStr for CommonRefFormatValues {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "page" => Ok(Self::Page),
            "chapter" => Ok(Self::Chapter),
            "direction" => Ok(Self::Direction),
            "text" => Ok(Self::Text),
            _ => Err(format!("unknown CommonRefFormatValues value: {}", s)),
        }
    }
}

pub type Coordinate = Length;

pub type CountryCode = String;

pub type Date = String;

pub type DateTime = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbDataSourceSettingTypes {
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "short")]
    Short,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "long")]
    Long,
    #[serde(rename = "double")]
    Double,
    #[serde(rename = "string")]
    String,
}

impl std::fmt::Display for DbDataSourceSettingTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean => write!(f, "boolean"),
            Self::Short => write!(f, "short"),
            Self::Int => write!(f, "int"),
            Self::Long => write!(f, "long"),
            Self::Double => write!(f, "double"),
            Self::String => write!(f, "string"),
        }
    }
}

impl std::str::FromStr for DbDataSourceSettingTypes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "boolean" => Ok(Self::Boolean),
            "short" => Ok(Self::Short),
            "int" => Ok(Self::Int),
            "long" => Ok(Self::Long),
            "double" => Ok(Self::Double),
            "string" => Ok(Self::String),
            _ => Err(format!("unknown DbDataSourceSettingTypes value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbDataTypes {
    #[serde(rename = "bit")]
    Bit,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "tinyint")]
    Tinyint,
    #[serde(rename = "smallint")]
    Smallint,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "bigint")]
    Bigint,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "real")]
    Real,
    #[serde(rename = "double")]
    Double,
    #[serde(rename = "numeric")]
    Numeric,
    #[serde(rename = "decimal")]
    Decimal,
    #[serde(rename = "char")]
    Char,
    #[serde(rename = "varchar")]
    Varchar,
    #[serde(rename = "longvarchar")]
    Longvarchar,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "timestmp")]
    Timestmp,
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "varbinary")]
    Varbinary,
    #[serde(rename = "longvarbinary")]
    Longvarbinary,
    #[serde(rename = "sqlnull")]
    Sqlnull,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "distinct")]
    Distinct,
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "blob")]
    Blob,
    #[serde(rename = "clob")]
    Clob,
    #[serde(rename = "ref")]
    Ref,
}

impl std::fmt::Display for DbDataTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bit => write!(f, "bit"),
            Self::Boolean => write!(f, "boolean"),
            Self::Tinyint => write!(f, "tinyint"),
            Self::Smallint => write!(f, "smallint"),
            Self::Integer => write!(f, "integer"),
            Self::Bigint => write!(f, "bigint"),
            Self::Float => write!(f, "float"),
            Self::Real => write!(f, "real"),
            Self::Double => write!(f, "double"),
            Self::Numeric => write!(f, "numeric"),
            Self::Decimal => write!(f, "decimal"),
            Self::Char => write!(f, "char"),
            Self::Varchar => write!(f, "varchar"),
            Self::Longvarchar => write!(f, "longvarchar"),
            Self::Date => write!(f, "date"),
            Self::Time => write!(f, "time"),
            Self::Timestmp => write!(f, "timestmp"),
            Self::Binary => write!(f, "binary"),
            Self::Varbinary => write!(f, "varbinary"),
            Self::Longvarbinary => write!(f, "longvarbinary"),
            Self::Sqlnull => write!(f, "sqlnull"),
            Self::Other => write!(f, "other"),
            Self::Object => write!(f, "object"),
            Self::Distinct => write!(f, "distinct"),
            Self::Struct => write!(f, "struct"),
            Self::Array => write!(f, "array"),
            Self::Blob => write!(f, "blob"),
            Self::Clob => write!(f, "clob"),
            Self::Ref => write!(f, "ref"),
        }
    }
}

impl std::str::FromStr for DbDataTypes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bit" => Ok(Self::Bit),
            "boolean" => Ok(Self::Boolean),
            "tinyint" => Ok(Self::Tinyint),
            "smallint" => Ok(Self::Smallint),
            "integer" => Ok(Self::Integer),
            "bigint" => Ok(Self::Bigint),
            "float" => Ok(Self::Float),
            "real" => Ok(Self::Real),
            "double" => Ok(Self::Double),
            "numeric" => Ok(Self::Numeric),
            "decimal" => Ok(Self::Decimal),
            "char" => Ok(Self::Char),
            "varchar" => Ok(Self::Varchar),
            "longvarchar" => Ok(Self::Longvarchar),
            "date" => Ok(Self::Date),
            "time" => Ok(Self::Time),
            "timestmp" => Ok(Self::Timestmp),
            "binary" => Ok(Self::Binary),
            "varbinary" => Ok(Self::Varbinary),
            "longvarbinary" => Ok(Self::Longvarbinary),
            "sqlnull" => Ok(Self::Sqlnull),
            "other" => Ok(Self::Other),
            "object" => Ok(Self::Object),
            "distinct" => Ok(Self::Distinct),
            "struct" => Ok(Self::Struct),
            "array" => Ok(Self::Array),
            "blob" => Ok(Self::Blob),
            "clob" => Ok(Self::Clob),
            "ref" => Ok(Self::Ref),
            _ => Err(format!("unknown DbDataTypes value: {}", s)),
        }
    }
}

pub type Distance = Length;

pub type Double = f64;

pub type Duration = String;

pub type ExtrusionOrigin = f64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontFamilyGeneric {
    #[serde(rename = "roman")]
    Roman,
    #[serde(rename = "swiss")]
    Swiss,
    #[serde(rename = "modern")]
    Modern,
    #[serde(rename = "decorative")]
    Decorative,
    #[serde(rename = "script")]
    Script,
    #[serde(rename = "system")]
    System,
}

impl std::fmt::Display for FontFamilyGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Roman => write!(f, "roman"),
            Self::Swiss => write!(f, "swiss"),
            Self::Modern => write!(f, "modern"),
            Self::Decorative => write!(f, "decorative"),
            Self::Script => write!(f, "script"),
            Self::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for FontFamilyGeneric {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "roman" => Ok(Self::Roman),
            "swiss" => Ok(Self::Swiss),
            "modern" => Ok(Self::Modern),
            "decorative" => Ok(Self::Decorative),
            "script" => Ok(Self::Script),
            "system" => Ok(Self::System),
            _ => Err(format!("unknown FontFamilyGeneric value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontPitch {
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "variable")]
    Variable,
}

impl std::fmt::Display for FontPitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed => write!(f, "fixed"),
            Self::Variable => write!(f, "variable"),
        }
    }
}

impl std::str::FromStr for FontPitch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fixed" => Ok(Self::Fixed),
            "variable" => Ok(Self::Variable),
            _ => Err(format!("unknown FontPitch value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "italic")]
    Italic,
    #[serde(rename = "oblique")]
    Oblique,
}

impl std::fmt::Display for FontStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Italic => write!(f, "italic"),
            Self::Oblique => write!(f, "oblique"),
        }
    }
}

impl std::str::FromStr for FontStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "italic" => Ok(Self::Italic),
            "oblique" => Ok(Self::Oblique),
            _ => Err(format!("unknown FontStyle value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontVariant {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "small-caps")]
    SmallCaps,
}

impl std::fmt::Display for FontVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::SmallCaps => write!(f, "small-caps"),
        }
    }
}

impl std::str::FromStr for FontVariant {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "small-caps" => Ok(Self::SmallCaps),
            _ => Err(format!("unknown FontVariant value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "bold")]
    Bold,
    #[serde(rename = "100")]
    _100,
    #[serde(rename = "200")]
    _200,
    #[serde(rename = "300")]
    _300,
    #[serde(rename = "400")]
    _400,
    #[serde(rename = "500")]
    _500,
    #[serde(rename = "600")]
    _600,
    #[serde(rename = "700")]
    _700,
    #[serde(rename = "800")]
    _800,
    #[serde(rename = "900")]
    _900,
}

impl std::fmt::Display for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Bold => write!(f, "bold"),
            Self::_100 => write!(f, "100"),
            Self::_200 => write!(f, "200"),
            Self::_300 => write!(f, "300"),
            Self::_400 => write!(f, "400"),
            Self::_500 => write!(f, "500"),
            Self::_600 => write!(f, "600"),
            Self::_700 => write!(f, "700"),
            Self::_800 => write!(f, "800"),
            Self::_900 => write!(f, "900"),
        }
    }
}

impl std::str::FromStr for FontWeight {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            "100" => Ok(Self::_100),
            "200" => Ok(Self::_200),
            "300" => Ok(Self::_300),
            "400" => Ok(Self::_400),
            "500" => Ok(Self::_500),
            "600" => Ok(Self::_600),
            "700" => Ok(Self::_700),
            "800" => Ok(Self::_800),
            "900" => Ok(Self::_900),
            _ => Err(format!("unknown FontWeight value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientStyle {
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "axial")]
    Axial,
    #[serde(rename = "radial")]
    Radial,
    #[serde(rename = "ellipsoid")]
    Ellipsoid,
    #[serde(rename = "square")]
    Square,
    #[serde(rename = "rectangular")]
    Rectangular,
}

impl std::fmt::Display for GradientStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Axial => write!(f, "axial"),
            Self::Radial => write!(f, "radial"),
            Self::Ellipsoid => write!(f, "ellipsoid"),
            Self::Square => write!(f, "square"),
            Self::Rectangular => write!(f, "rectangular"),
        }
    }
}

impl std::str::FromStr for GradientStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linear" => Ok(Self::Linear),
            "axial" => Ok(Self::Axial),
            "radial" => Ok(Self::Radial),
            "ellipsoid" => Ok(Self::Ellipsoid),
            "square" => Ok(Self::Square),
            "rectangular" => Ok(Self::Rectangular),
            _ => Err(format!("unknown GradientStyle value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoriBackPos {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "right")]
    Right,
}

impl std::fmt::Display for HoriBackPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Center => write!(f, "center"),
            Self::Right => write!(f, "right"),
        }
    }
}

impl std::str::FromStr for HoriBackPos {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(format!("unknown HoriBackPos value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizontalMirror {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "horizontal-on-odd")]
    HorizontalOnOdd,
    #[serde(rename = "horizontal-on-even")]
    HorizontalOnEven,
}

impl std::fmt::Display for HorizontalMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Horizontal => write!(f, "horizontal"),
            Self::HorizontalOnOdd => write!(f, "horizontal-on-odd"),
            Self::HorizontalOnEven => write!(f, "horizontal-on-even"),
        }
    }
}

impl std::str::FromStr for HorizontalMirror {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "horizontal" => Ok(Self::Horizontal),
            "horizontal-on-odd" => Ok(Self::HorizontalOnOdd),
            "horizontal-on-even" => Ok(Self::HorizontalOnEven),
            _ => Err(format!("unknown HorizontalMirror value: {}", s)),
        }
    }
}

pub type Integer = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelPositions {
    #[serde(rename = "avoid-overlap")]
    AvoidOverlap,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "top-right")]
    TopRight,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "bottom-right")]
    BottomRight,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "bottom-left")]
    BottomLeft,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "top-left")]
    TopLeft,
    #[serde(rename = "inside")]
    Inside,
    #[serde(rename = "outside")]
    Outside,
    #[serde(rename = "near-origin")]
    NearOrigin,
}

impl std::fmt::Display for LabelPositions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AvoidOverlap => write!(f, "avoid-overlap"),
            Self::Center => write!(f, "center"),
            Self::Top => write!(f, "top"),
            Self::TopRight => write!(f, "top-right"),
            Self::Right => write!(f, "right"),
            Self::BottomRight => write!(f, "bottom-right"),
            Self::Bottom => write!(f, "bottom"),
            Self::BottomLeft => write!(f, "bottom-left"),
            Self::Left => write!(f, "left"),
            Self::TopLeft => write!(f, "top-left"),
            Self::Inside => write!(f, "inside"),
            Self::Outside => write!(f, "outside"),
            Self::NearOrigin => write!(f, "near-origin"),
        }
    }
}

impl std::str::FromStr for LabelPositions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "avoid-overlap" => Ok(Self::AvoidOverlap),
            "center" => Ok(Self::Center),
            "top" => Ok(Self::Top),
            "top-right" => Ok(Self::TopRight),
            "right" => Ok(Self::Right),
            "bottom-right" => Ok(Self::BottomRight),
            "bottom" => Ok(Self::Bottom),
            "bottom-left" => Ok(Self::BottomLeft),
            "left" => Ok(Self::Left),
            "top-left" => Ok(Self::TopLeft),
            "inside" => Ok(Self::Inside),
            "outside" => Ok(Self::Outside),
            "near-origin" => Ok(Self::NearOrigin),
            _ => Err(format!("unknown LabelPositions value: {}", s)),
        }
    }
}

pub type Language = String;

pub type LanguageCode = String;

pub type Length = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineMode {
    #[serde(rename = "continuous")]
    Continuous,
    #[serde(rename = "skip-white-space")]
    SkipWhiteSpace,
}

impl std::fmt::Display for LineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Continuous => write!(f, "continuous"),
            Self::SkipWhiteSpace => write!(f, "skip-white-space"),
        }
    }
}

impl std::str::FromStr for LineMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "continuous" => Ok(Self::Continuous),
            "skip-white-space" => Ok(Self::SkipWhiteSpace),
            _ => Err(format!("unknown LineMode value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "solid")]
    Solid,
    #[serde(rename = "dotted")]
    Dotted,
    #[serde(rename = "dash")]
    Dash,
    #[serde(rename = "long-dash")]
    LongDash,
    #[serde(rename = "dot-dash")]
    DotDash,
    #[serde(rename = "dot-dot-dash")]
    DotDotDash,
    #[serde(rename = "wave")]
    Wave,
}

impl std::fmt::Display for LineStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Solid => write!(f, "solid"),
            Self::Dotted => write!(f, "dotted"),
            Self::Dash => write!(f, "dash"),
            Self::LongDash => write!(f, "long-dash"),
            Self::DotDash => write!(f, "dot-dash"),
            Self::DotDotDash => write!(f, "dot-dot-dash"),
            Self::Wave => write!(f, "wave"),
        }
    }
}

impl std::str::FromStr for LineStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "solid" => Ok(Self::Solid),
            "dotted" => Ok(Self::Dotted),
            "dash" => Ok(Self::Dash),
            "long-dash" => Ok(Self::LongDash),
            "dot-dash" => Ok(Self::DotDash),
            "dot-dot-dash" => Ok(Self::DotDotDash),
            "wave" => Ok(Self::Wave),
            _ => Err(format!("unknown LineStyle value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "single")]
    Single,
    #[serde(rename = "double")]
    Double,
}

impl std::fmt::Display for LineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Single => write!(f, "single"),
            Self::Double => write!(f, "double"),
        }
    }
}

impl std::str::FromStr for LineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            _ => Err(format!("unknown LineType value: {}", s)),
        }
    }
}

pub type NamespacedToken = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Navigation {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "current")]
    Current,
    #[serde(rename = "parent")]
    Parent,
}

impl std::fmt::Display for Navigation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Current => write!(f, "current"),
            Self::Parent => write!(f, "parent"),
        }
    }
}

impl std::str::FromStr for Navigation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "current" => Ok(Self::Current),
            "parent" => Ok(Self::Parent),
            _ => Err(format!("unknown Navigation value: {}", s)),
        }
    }
}

pub type NonNegativeDecimal = f64;

pub type NonNegativeInteger = String;

pub type NonNegativeLength = String;

pub type NonNegativePixelLength = String;

pub type PathData = String;

pub type Percent = String;

pub type Point3D = String;

pub type Points = String;

pub type PositiveInteger = String;

pub type PositiveLength = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresentationClasses {
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "outline")]
    Outline,
    #[serde(rename = "subtitle")]
    Subtitle,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "graphic")]
    Graphic,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "chart")]
    Chart,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "orgchart")]
    Orgchart,
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "notes")]
    Notes,
    #[serde(rename = "handout")]
    Handout,
    #[serde(rename = "header")]
    Header,
    #[serde(rename = "footer")]
    Footer,
    #[serde(rename = "date-time")]
    DateTime,
    #[serde(rename = "page-number")]
    PageNumber,
}

impl std::fmt::Display for PresentationClasses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Title => write!(f, "title"),
            Self::Outline => write!(f, "outline"),
            Self::Subtitle => write!(f, "subtitle"),
            Self::Text => write!(f, "text"),
            Self::Graphic => write!(f, "graphic"),
            Self::Object => write!(f, "object"),
            Self::Chart => write!(f, "chart"),
            Self::Table => write!(f, "table"),
            Self::Orgchart => write!(f, "orgchart"),
            Self::Page => write!(f, "page"),
            Self::Notes => write!(f, "notes"),
            Self::Handout => write!(f, "handout"),
            Self::Header => write!(f, "header"),
            Self::Footer => write!(f, "footer"),
            Self::DateTime => write!(f, "date-time"),
            Self::PageNumber => write!(f, "page-number"),
        }
    }
}

impl std::str::FromStr for PresentationClasses {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "title" => Ok(Self::Title),
            "outline" => Ok(Self::Outline),
            "subtitle" => Ok(Self::Subtitle),
            "text" => Ok(Self::Text),
            "graphic" => Ok(Self::Graphic),
            "object" => Ok(Self::Object),
            "chart" => Ok(Self::Chart),
            "table" => Ok(Self::Table),
            "orgchart" => Ok(Self::Orgchart),
            "page" => Ok(Self::Page),
            "notes" => Ok(Self::Notes),
            "handout" => Ok(Self::Handout),
            "header" => Ok(Self::Header),
            "footer" => Ok(Self::Footer),
            "date-time" => Ok(Self::DateTime),
            "page-number" => Ok(Self::PageNumber),
            _ => Err(format!("unknown PresentationClasses value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresentationEffectDirections {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "from-left")]
    FromLeft,
    #[serde(rename = "from-top")]
    FromTop,
    #[serde(rename = "from-right")]
    FromRight,
    #[serde(rename = "from-bottom")]
    FromBottom,
    #[serde(rename = "from-center")]
    FromCenter,
    #[serde(rename = "from-upper-left")]
    FromUpperLeft,
    #[serde(rename = "from-upper-right")]
    FromUpperRight,
    #[serde(rename = "from-lower-left")]
    FromLowerLeft,
    #[serde(rename = "from-lower-right")]
    FromLowerRight,
    #[serde(rename = "to-left")]
    ToLeft,
    #[serde(rename = "to-top")]
    ToTop,
    #[serde(rename = "to-right")]
    ToRight,
    #[serde(rename = "to-bottom")]
    ToBottom,
    #[serde(rename = "to-upper-left")]
    ToUpperLeft,
    #[serde(rename = "to-upper-right")]
    ToUpperRight,
    #[serde(rename = "to-lower-right")]
    ToLowerRight,
    #[serde(rename = "to-lower-left")]
    ToLowerLeft,
    #[serde(rename = "path")]
    Path,
    #[serde(rename = "spiral-inward-left")]
    SpiralInwardLeft,
    #[serde(rename = "spiral-inward-right")]
    SpiralInwardRight,
    #[serde(rename = "spiral-outward-left")]
    SpiralOutwardLeft,
    #[serde(rename = "spiral-outward-right")]
    SpiralOutwardRight,
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "to-center")]
    ToCenter,
    #[serde(rename = "clockwise")]
    Clockwise,
    #[serde(rename = "counter-clockwise")]
    CounterClockwise,
}

impl std::fmt::Display for PresentationEffectDirections {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::FromLeft => write!(f, "from-left"),
            Self::FromTop => write!(f, "from-top"),
            Self::FromRight => write!(f, "from-right"),
            Self::FromBottom => write!(f, "from-bottom"),
            Self::FromCenter => write!(f, "from-center"),
            Self::FromUpperLeft => write!(f, "from-upper-left"),
            Self::FromUpperRight => write!(f, "from-upper-right"),
            Self::FromLowerLeft => write!(f, "from-lower-left"),
            Self::FromLowerRight => write!(f, "from-lower-right"),
            Self::ToLeft => write!(f, "to-left"),
            Self::ToTop => write!(f, "to-top"),
            Self::ToRight => write!(f, "to-right"),
            Self::ToBottom => write!(f, "to-bottom"),
            Self::ToUpperLeft => write!(f, "to-upper-left"),
            Self::ToUpperRight => write!(f, "to-upper-right"),
            Self::ToLowerRight => write!(f, "to-lower-right"),
            Self::ToLowerLeft => write!(f, "to-lower-left"),
            Self::Path => write!(f, "path"),
            Self::SpiralInwardLeft => write!(f, "spiral-inward-left"),
            Self::SpiralInwardRight => write!(f, "spiral-inward-right"),
            Self::SpiralOutwardLeft => write!(f, "spiral-outward-left"),
            Self::SpiralOutwardRight => write!(f, "spiral-outward-right"),
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::ToCenter => write!(f, "to-center"),
            Self::Clockwise => write!(f, "clockwise"),
            Self::CounterClockwise => write!(f, "counter-clockwise"),
        }
    }
}

impl std::str::FromStr for PresentationEffectDirections {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "from-left" => Ok(Self::FromLeft),
            "from-top" => Ok(Self::FromTop),
            "from-right" => Ok(Self::FromRight),
            "from-bottom" => Ok(Self::FromBottom),
            "from-center" => Ok(Self::FromCenter),
            "from-upper-left" => Ok(Self::FromUpperLeft),
            "from-upper-right" => Ok(Self::FromUpperRight),
            "from-lower-left" => Ok(Self::FromLowerLeft),
            "from-lower-right" => Ok(Self::FromLowerRight),
            "to-left" => Ok(Self::ToLeft),
            "to-top" => Ok(Self::ToTop),
            "to-right" => Ok(Self::ToRight),
            "to-bottom" => Ok(Self::ToBottom),
            "to-upper-left" => Ok(Self::ToUpperLeft),
            "to-upper-right" => Ok(Self::ToUpperRight),
            "to-lower-right" => Ok(Self::ToLowerRight),
            "to-lower-left" => Ok(Self::ToLowerLeft),
            "path" => Ok(Self::Path),
            "spiral-inward-left" => Ok(Self::SpiralInwardLeft),
            "spiral-inward-right" => Ok(Self::SpiralInwardRight),
            "spiral-outward-left" => Ok(Self::SpiralOutwardLeft),
            "spiral-outward-right" => Ok(Self::SpiralOutwardRight),
            "vertical" => Ok(Self::Vertical),
            "horizontal" => Ok(Self::Horizontal),
            "to-center" => Ok(Self::ToCenter),
            "clockwise" => Ok(Self::Clockwise),
            "counter-clockwise" => Ok(Self::CounterClockwise),
            _ => Err(format!("unknown PresentationEffectDirections value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresentationEffects {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "fade")]
    Fade,
    #[serde(rename = "move")]
    Move,
    #[serde(rename = "stripes")]
    Stripes,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "dissolve")]
    Dissolve,
    #[serde(rename = "wavyline")]
    Wavyline,
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "lines")]
    Lines,
    #[serde(rename = "laser")]
    Laser,
    #[serde(rename = "appear")]
    Appear,
    #[serde(rename = "hide")]
    Hide,
    #[serde(rename = "move-short")]
    MoveShort,
    #[serde(rename = "checkerboard")]
    Checkerboard,
    #[serde(rename = "rotate")]
    Rotate,
    #[serde(rename = "stretch")]
    Stretch,
}

impl std::fmt::Display for PresentationEffects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Fade => write!(f, "fade"),
            Self::Move => write!(f, "move"),
            Self::Stripes => write!(f, "stripes"),
            Self::Open => write!(f, "open"),
            Self::Close => write!(f, "close"),
            Self::Dissolve => write!(f, "dissolve"),
            Self::Wavyline => write!(f, "wavyline"),
            Self::Random => write!(f, "random"),
            Self::Lines => write!(f, "lines"),
            Self::Laser => write!(f, "laser"),
            Self::Appear => write!(f, "appear"),
            Self::Hide => write!(f, "hide"),
            Self::MoveShort => write!(f, "move-short"),
            Self::Checkerboard => write!(f, "checkerboard"),
            Self::Rotate => write!(f, "rotate"),
            Self::Stretch => write!(f, "stretch"),
        }
    }
}

impl std::str::FromStr for PresentationEffects {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "fade" => Ok(Self::Fade),
            "move" => Ok(Self::Move),
            "stripes" => Ok(Self::Stripes),
            "open" => Ok(Self::Open),
            "close" => Ok(Self::Close),
            "dissolve" => Ok(Self::Dissolve),
            "wavyline" => Ok(Self::Wavyline),
            "random" => Ok(Self::Random),
            "lines" => Ok(Self::Lines),
            "laser" => Ok(Self::Laser),
            "appear" => Ok(Self::Appear),
            "hide" => Ok(Self::Hide),
            "move-short" => Ok(Self::MoveShort),
            "checkerboard" => Ok(Self::Checkerboard),
            "rotate" => Ok(Self::Rotate),
            "stretch" => Ok(Self::Stretch),
            _ => Err(format!("unknown PresentationEffects value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresentationSpeeds {
    #[serde(rename = "slow")]
    Slow,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "fast")]
    Fast,
}

impl std::fmt::Display for PresentationSpeeds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Medium => write!(f, "medium"),
            Self::Fast => write!(f, "fast"),
        }
    }
}

impl std::str::FromStr for PresentationSpeeds {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "slow" => Ok(Self::Slow),
            "medium" => Ok(Self::Medium),
            "fast" => Ok(Self::Fast),
            _ => Err(format!("unknown PresentationSpeeds value: {}", s)),
        }
    }
}

pub type RelativeLength = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowOrCol {
    #[serde(rename = "row")]
    Row,
    #[serde(rename = "column")]
    Column,
}

impl std::fmt::Display for RowOrCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row => write!(f, "row"),
            Self::Column => write!(f, "column"),
        }
    }
}

impl std::str::FromStr for RowOrCol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "row" => Ok(Self::Row),
            "column" => Ok(Self::Column),
            _ => Err(format!("unknown RowOrCol value: {}", s)),
        }
    }
}

pub type ScriptCode = String;

pub type SignedZeroToHundredPercent = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum States {
    #[serde(rename = "unchecked")]
    Unchecked,
    #[serde(rename = "checked")]
    Checked,
    #[serde(rename = "unknown")]
    Unknown,
}

impl std::fmt::Display for States {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unchecked => write!(f, "unchecked"),
            Self::Checked => write!(f, "checked"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for States {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unchecked" => Ok(Self::Unchecked),
            "checked" => Ok(Self::Checked),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("unknown States value: {}", s)),
        }
    }
}

pub type StyleName = String;

pub type StyleNameRefs = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabCycles {
    #[serde(rename = "records")]
    Records,
    #[serde(rename = "current")]
    Current,
    #[serde(rename = "page")]
    Page,
}

impl std::fmt::Display for TabCycles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Records => write!(f, "records"),
            Self::Current => write!(f, "current"),
            Self::Page => write!(f, "page"),
        }
    }
}

impl std::str::FromStr for TabCycles {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "records" => Ok(Self::Records),
            "current" => Ok(Self::Current),
            "page" => Ok(Self::Page),
            _ => Err(format!("unknown TabCycles value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableVisibilityValue {
    #[serde(rename = "visible")]
    Visible,
    #[serde(rename = "collapse")]
    Collapse,
    #[serde(rename = "filter")]
    Filter,
}

impl std::fmt::Display for TableVisibilityValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Visible => write!(f, "visible"),
            Self::Collapse => write!(f, "collapse"),
            Self::Filter => write!(f, "filter"),
        }
    }
}

impl std::str::FromStr for TableVisibilityValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "visible" => Ok(Self::Visible),
            "collapse" => Ok(Self::Collapse),
            "filter" => Ok(Self::Filter),
            _ => Err(format!("unknown TableVisibilityValue value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextBibliographyTypes {
    #[serde(rename = "article")]
    Article,
    #[serde(rename = "book")]
    Book,
    #[serde(rename = "booklet")]
    Booklet,
    #[serde(rename = "conference")]
    Conference,
    #[serde(rename = "custom1")]
    Custom1,
    #[serde(rename = "custom2")]
    Custom2,
    #[serde(rename = "custom3")]
    Custom3,
    #[serde(rename = "custom4")]
    Custom4,
    #[serde(rename = "custom5")]
    Custom5,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "inbook")]
    Inbook,
    #[serde(rename = "incollection")]
    Incollection,
    #[serde(rename = "inproceedings")]
    Inproceedings,
    #[serde(rename = "journal")]
    Journal,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "mastersthesis")]
    Mastersthesis,
    #[serde(rename = "misc")]
    Misc,
    #[serde(rename = "phdthesis")]
    Phdthesis,
    #[serde(rename = "proceedings")]
    Proceedings,
    #[serde(rename = "techreport")]
    Techreport,
    #[serde(rename = "unpublished")]
    Unpublished,
    #[serde(rename = "www")]
    Www,
}

impl std::fmt::Display for TextBibliographyTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Article => write!(f, "article"),
            Self::Book => write!(f, "book"),
            Self::Booklet => write!(f, "booklet"),
            Self::Conference => write!(f, "conference"),
            Self::Custom1 => write!(f, "custom1"),
            Self::Custom2 => write!(f, "custom2"),
            Self::Custom3 => write!(f, "custom3"),
            Self::Custom4 => write!(f, "custom4"),
            Self::Custom5 => write!(f, "custom5"),
            Self::Email => write!(f, "email"),
            Self::Inbook => write!(f, "inbook"),
            Self::Incollection => write!(f, "incollection"),
            Self::Inproceedings => write!(f, "inproceedings"),
            Self::Journal => write!(f, "journal"),
            Self::Manual => write!(f, "manual"),
            Self::Mastersthesis => write!(f, "mastersthesis"),
            Self::Misc => write!(f, "misc"),
            Self::Phdthesis => write!(f, "phdthesis"),
            Self::Proceedings => write!(f, "proceedings"),
            Self::Techreport => write!(f, "techreport"),
            Self::Unpublished => write!(f, "unpublished"),
            Self::Www => write!(f, "www"),
        }
    }
}

impl std::str::FromStr for TextBibliographyTypes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "article" => Ok(Self::Article),
            "book" => Ok(Self::Book),
            "booklet" => Ok(Self::Booklet),
            "conference" => Ok(Self::Conference),
            "custom1" => Ok(Self::Custom1),
            "custom2" => Ok(Self::Custom2),
            "custom3" => Ok(Self::Custom3),
            "custom4" => Ok(Self::Custom4),
            "custom5" => Ok(Self::Custom5),
            "email" => Ok(Self::Email),
            "inbook" => Ok(Self::Inbook),
            "incollection" => Ok(Self::Incollection),
            "inproceedings" => Ok(Self::Inproceedings),
            "journal" => Ok(Self::Journal),
            "manual" => Ok(Self::Manual),
            "mastersthesis" => Ok(Self::Mastersthesis),
            "misc" => Ok(Self::Misc),
            "phdthesis" => Ok(Self::Phdthesis),
            "proceedings" => Ok(Self::Proceedings),
            "techreport" => Ok(Self::Techreport),
            "unpublished" => Ok(Self::Unpublished),
            "www" => Ok(Self::Www),
            _ => Err(format!("unknown TextBibliographyTypes value: {}", s)),
        }
    }
}

pub type TextEncoding = String;

pub type Time = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Types {
    #[serde(rename = "submit")]
    Submit,
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "url")]
    Url,
}

impl std::fmt::Display for Types {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submit => write!(f, "submit"),
            Self::Reset => write!(f, "reset"),
            Self::Push => write!(f, "push"),
            Self::Url => write!(f, "url"),
        }
    }
}

impl std::str::FromStr for Types {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "submit" => Ok(Self::Submit),
            "reset" => Ok(Self::Reset),
            "push" => Ok(Self::Push),
            "url" => Ok(Self::Url),
            _ => Err(format!("unknown Types value: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "currency")]
    Currency,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "string")]
    String,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float => write!(f, "float"),
            Self::Time => write!(f, "time"),
            Self::Date => write!(f, "date"),
            Self::Percentage => write!(f, "percentage"),
            Self::Currency => write!(f, "currency"),
            Self::Boolean => write!(f, "boolean"),
            Self::String => write!(f, "string"),
        }
    }
}

impl std::str::FromStr for ValueType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "float" => Ok(Self::Float),
            "time" => Ok(Self::Time),
            "date" => Ok(Self::Date),
            "percentage" => Ok(Self::Percentage),
            "currency" => Ok(Self::Currency),
            "boolean" => Ok(Self::Boolean),
            "string" => Ok(Self::String),
            _ => Err(format!("unknown ValueType value: {}", s)),
        }
    }
}

pub type VariableName = String;

pub type Vector3D = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VertBackPos {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "bottom")]
    Bottom,
}

impl std::fmt::Display for VertBackPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Top => write!(f, "top"),
            Self::Center => write!(f, "center"),
            Self::Bottom => write!(f, "bottom"),
        }
    }
}

impl std::str::FromStr for VertBackPos {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(format!("unknown VertBackPos value: {}", s)),
        }
    }
}

pub type ZeroToHundredPercent = String;

pub type ZeroToOneDecimal = f64;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Start {
    #[serde(rename = "document")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    #[serde(rename = "document-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_content: Option<String>,
    #[serde(rename = "document-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_styles: Option<String>,
    #[serde(rename = "document-meta")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_meta: Option<String>,
    #[serde(rename = "document-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_settings: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct URIorSafeCURIE {
    #[serde(rename = "$text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimAnimateColorAttlist {
    #[serde(rename = "@anim:color-interpolation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_interpolation: Option<String>,
    #[serde(rename = "@anim:color-interpolation-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_interpolation_direction: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimAnimateMotionAttlist {
    #[serde(rename = "@svg:path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathData>,
    #[serde(rename = "@svg:origin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(rename = "@smil:calcMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calc_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimAnimateTransformAttlist {
    #[serde(rename = "@svg:type")]
    pub r#type: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimAudioAttlist {
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@anim:audio-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_level: Option<Double>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimCommandAttlist {
    #[serde(rename = "@anim:command")]
    pub command: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimIterateAttlist {
    #[serde(rename = "@smil:targetElement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_element: Option<IDREF>,
    #[serde(rename = "@anim:sub-item")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_item: Option<String>,
    #[serde(rename = "@anim:iterate-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterate_type: Option<String>,
    #[serde(rename = "@anim:iterate-interval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterate_interval: Option<Duration>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimTransitionFilterAttlist {
    #[serde(rename = "@smil:type")]
    pub r#type: String,
    #[serde(rename = "@smil:subtype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(rename = "@smil:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@smil:fadeColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_color: Option<Color>,
    #[serde(rename = "@smil:mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimationElement {
    #[serde(rename = "animate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate: Option<String>,
    #[serde(rename = "set")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<String>,
    #[serde(rename = "animateMotion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate_motion: Option<String>,
    #[serde(rename = "animateColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate_color: Option<String>,
    #[serde(rename = "animateTransform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate_transform: Option<String>,
    #[serde(rename = "transitionFilter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_filter: Option<String>,
    #[serde(rename = "par")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub par: Option<String>,
    #[serde(rename = "seq")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<String>,
    #[serde(rename = "iterate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterate: Option<String>,
    #[serde(rename = "audio")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(rename = "command")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnyDate {
    #[serde(rename = "day")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    #[serde(rename = "month")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
    #[serde(rename = "year")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(rename = "era")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
    #[serde(rename = "day-of-week")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_of_week: Option<String>,
    #[serde(rename = "week-of-year")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub week_of_year: Option<String>,
    #[serde(rename = "quarter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarter: Option<String>,
    #[serde(rename = "hours")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<String>,
    #[serde(rename = "am-pm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub am_pm: Option<()>,
    #[serde(rename = "minutes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<String>,
    #[serde(rename = "seconds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnyNumber {
    #[serde(rename = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(rename = "scientific-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scientific_number: Option<String>,
    #[serde(rename = "fraction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fraction: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnyTime {
    #[serde(rename = "hours")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<String>,
    #[serde(rename = "am-pm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub am_pm: Option<()>,
    #[serde(rename = "minutes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<String>,
    #[serde(rename = "seconds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnyAttListOrElements {
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnyElements {
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundColumn {
    #[serde(rename = "@form:bound-column")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_column: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ButtonType {
    #[serde(rename = "@form:button-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_type: Option<Types>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellRangeAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMarkAttr {
    #[serde(rename = "@text:change-id")]
    pub change_id: IDREF,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeMarks {
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type ChartAxis = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartAxisAttlist {
    #[serde(rename = "@chart:dimension")]
    pub dimension: ChartDimension,
    #[serde(rename = "@chart:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartCategories = String;

pub type ChartChart = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartChartAttlist {
    #[serde(rename = "@chart:class")]
    pub class: NamespacedToken,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@chart:column-mapping")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_mapping: Option<String>,
    #[serde(rename = "@chart:row-mapping")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_mapping: Option<String>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartCoordinateRegion = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartCoordinateRegionAttlist {
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartDataLabel = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartDataLabelAttlist {
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartDataPoint = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartDataPointAttlist {
    #[serde(rename = "@chart:repeated")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeated: Option<PositiveInteger>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartDomain = String;

pub type ChartEquation = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartEquationAttlist {
    #[serde(rename = "@chart:automatic-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_content: Option<Boolean>,
    #[serde(rename = "@chart:display-r-square")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_r_square: Option<Boolean>,
    #[serde(rename = "@chart:display-equation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_equation: Option<Boolean>,
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartErrorIndicator = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartErrorIndicatorAttlist {
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@chart:dimension")]
    pub dimension: ChartDimension,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartFloor = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartFloorAttlist {
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartFooter = String;

pub type ChartGrid = ChartGridAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartGridAttlist {
    #[serde(rename = "@chart:class")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartLegend = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartLegendAttlist {
    #[serde(rename = "@chart:legend-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<String>,
    #[serde(rename = "@chart:legend-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_align: Option<String>,
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@style:legend-expansion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_expansion: Option<String>,
    #[serde(rename = "@style:legend-expansion-aspect-ratio")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_expansion_aspect_ratio: Option<Double>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartMeanValue = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartMeanValueAttlist {
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartPlotArea = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartPlotAreaAttlist {
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:cell-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_range_address: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:data-source-has-labels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source_has_labels: Option<String>,
    #[serde(rename = "@dr3d:vrp")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vrp: Option<Vector3D>,
    #[serde(rename = "@dr3d:vpn")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn: Option<Vector3D>,
    #[serde(rename = "@dr3d:vup")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vup: Option<Vector3D>,
    #[serde(rename = "@dr3d:projection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
    #[serde(rename = "@dr3d:distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<Length>,
    #[serde(rename = "@dr3d:focal-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<Length>,
    #[serde(rename = "@dr3d:shadow-slant")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_slant: Option<Angle>,
    #[serde(rename = "@dr3d:shade-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shade_mode: Option<String>,
    #[serde(rename = "@dr3d:ambient-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<Color>,
    #[serde(rename = "@dr3d:lighting-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting_mode: Option<Boolean>,
    #[serde(rename = "@dr3d:transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartRegressionCurve = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartRegressionCurveAttlist {
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartSeries = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartSeriesAttlist {
    #[serde(rename = "@chart:values-cell-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values_cell_range_address: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:label-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_cell_address: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:class")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<NamespacedToken>,
    #[serde(rename = "@chart:attached-axis")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_axis: Option<String>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartStockGainMarker = CommonStockMarkerAttlist;

pub type ChartStockLossMarker = CommonStockMarkerAttlist;

pub type ChartStockRangeLine = CommonStockMarkerAttlist;

pub type ChartSubtitle = String;

pub type ChartTitle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartTitleAttlist {
    #[serde(rename = "@table:cell-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_range: Option<CellRangeAddressList>,
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ChartWall = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartWallAttlist {
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnControls {
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "textarea")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea: Option<String>,
    #[serde(rename = "formatted-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_text: Option<String>,
    #[serde(rename = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(rename = "combobox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combobox: Option<String>,
    #[serde(rename = "listbox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listbox: Option<String>,
    #[serde(rename = "checkbox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkbox: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimAddAccumAttlist {
    #[serde(rename = "@smil:accumulate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accumulate: Option<String>,
    #[serde(rename = "@smil:additive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additive: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimAttlist {
    #[serde(rename = "@presentation:node-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    #[serde(rename = "@presentation:preset-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(rename = "@presentation:preset-sub-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_sub_type: Option<String>,
    #[serde(rename = "@presentation:preset-class")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_class: Option<String>,
    #[serde(rename = "@presentation:master-element")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_element: Option<IDREF>,
    #[serde(rename = "@presentation:group-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonAnimNamedTargetAttlist {
    #[serde(rename = "@smil:attributeName")]
    pub attribute_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimSetValuesAttlist {
    #[serde(rename = "@smil:to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimSplineModeAttlist {
    #[serde(rename = "@smil:calcMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calc_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimTargetAttlist {
    #[serde(rename = "@smil:targetElement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_element: Option<IDREF>,
    #[serde(rename = "@anim:sub-item")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_item: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAnimValuesAttlist {
    #[serde(rename = "@smil:values")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<String>,
    #[serde(rename = "@anim:formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(rename = "@smil:to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(rename = "@smil:from")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(rename = "@smil:by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonAutoReorderAttlist {
    #[serde(rename = "@number:automatic-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_order: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBackgroundColorAttlist {
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBackgroundTransparencyAttlist {
    #[serde(rename = "@style:background-transparency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<ZeroToHundredPercent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBasicTimingAttlist {
    #[serde(rename = "@smil:begin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    #[serde(rename = "@smil:end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(rename = "@smil:dur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dur: Option<String>,
    #[serde(rename = "@smil:repeatDur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_dur: Option<String>,
    #[serde(rename = "@smil:repeatCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<String>,
    #[serde(rename = "@smil:restart")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(rename = "@smil:restartDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_default: Option<String>,
    #[serde(rename = "@smil:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(rename = "@smil:fillDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_default: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBeginEndTimingAttlist {
    #[serde(rename = "@smil:begin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    #[serde(rename = "@smil:end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBorderAttlist {
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBorderLineWidthAttlist {
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonBreakAttlist {
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonCalendarAttlist {
    #[serde(rename = "@number:calendar")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonContourAttlist {
    #[serde(rename = "@draw:recreate-on-edit")]
    pub recreate_on_edit: Boolean,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonControlIdAttlist {
    #[serde(rename = "@xml:id")]
    pub id: ID,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonConvertEmptyAttlist {
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonCurrentValueAttlist {
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDataFieldAttlist {
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDataStyleAttlist {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    #[serde(rename = "@style:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@number:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@number:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@number:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@number:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@number:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@style:volatile")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volatile: Option<Boolean>,
    #[serde(rename = "@number:transliteration-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transliteration_format: Option<String>,
    #[serde(rename = "@number:transliteration-language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transliteration_language: Option<CountryCode>,
    #[serde(rename = "@number:transliteration-country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transliteration_country: Option<CountryCode>,
    #[serde(rename = "@number:transliteration-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transliteration_style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDbDefaultValue {
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "@office:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@office:currency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "@office:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@office:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<Duration>,
    #[serde(rename = "@office:boolean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<Boolean>,
    #[serde(rename = "@office:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDbObjectDescription {
    #[serde(rename = "@db:description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDbObjectName {
    #[serde(rename = "@db:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDbObjectTitle {
    #[serde(rename = "@db:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDbTableNameAttlist {
    #[serde(rename = "@db:name")]
    pub name: String,
    #[serde(rename = "@db:catalog-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(rename = "@db:schema-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDbTableStyleName {
    #[serde(rename = "@db:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@db:default-row-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_row_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDdeConnectionDeclAttlist {
    #[serde(rename = "@office:dde-application")]
    pub dde_application: String,
    #[serde(rename = "@office:dde-topic")]
    pub dde_topic: String,
    #[serde(rename = "@office:dde-item")]
    pub dde_item: String,
    #[serde(rename = "@office:automatic-update")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_update: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDecimalPlacesAttlist {
    #[serde(rename = "@number:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<Integer>,
    #[serde(rename = "@number:min-decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_decimal_places: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDelayForRepeat {
    #[serde(rename = "@form:delay-for-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_for_repeat: Option<Duration>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDisabledAttlist {
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDr3dTransformAttlist {
    #[serde(rename = "@dr3d:transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawAreaAttlist {
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@office:target-frame-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame_name: Option<TargetFrameName>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@office:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@draw:nohref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nohref: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawCaptionIdAttlist {
    #[serde(rename = "@draw:caption-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_id: Option<IDREF>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawCircleEllipseAttlist {
    #[serde(rename = "@draw:kind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "@draw:start-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_angle: Option<Angle>,
    #[serde(rename = "@draw:end-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_angle: Option<Angle>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawCircleEllipsePosAttlist {
    #[serde(rename = "@svg:cx")]
    pub cx: Coordinate,
    #[serde(rename = "@svg:cy")]
    pub cy: Coordinate,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawDataAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawGradientAttlist {
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<StyleName>,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@draw:style")]
    pub style: GradientStyle,
    #[serde(rename = "@draw:cx")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx: Option<Percent>,
    #[serde(rename = "@draw:cy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cy: Option<Percent>,
    #[serde(rename = "@draw:angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<Angle>,
    #[serde(rename = "@draw:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Percent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawIdAttlist {
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawLayerNameAttlist {
    #[serde(rename = "@draw:layer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawMimeTypeAttlist {
    #[serde(rename = "@draw:mime-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawNameAttlist {
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawPathDataAttlist {
    #[serde(rename = "@svg:d")]
    pub d: PathData,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawPointsAttlist {
    #[serde(rename = "@draw:points")]
    pub points: Points,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawPositionAttlist {
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawRelSizeAttlist {
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<String>,
    #[serde(rename = "@style:rel-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_height: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawShapeWithStylesAttlist {
    #[serde(rename = "@draw:z-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_index: Option<NonNegativeInteger>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@draw:layer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:class-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_names: Option<StyleNameRefs>,
    #[serde(rename = "@draw:transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@table:end-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_cell_address: Option<CellAddress>,
    #[serde(rename = "@table:end-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_x: Option<Coordinate>,
    #[serde(rename = "@table:end-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_y: Option<Coordinate>,
    #[serde(rename = "@table:table-background")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_background: Option<Boolean>,
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawShapeWithTextAndStylesAttlist {
    #[serde(rename = "@draw:z-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_index: Option<NonNegativeInteger>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@draw:layer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:class-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_names: Option<StyleNameRefs>,
    #[serde(rename = "@draw:transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@table:end-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_cell_address: Option<CellAddress>,
    #[serde(rename = "@table:end-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_x: Option<Coordinate>,
    #[serde(rename = "@table:end-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_y: Option<Coordinate>,
    #[serde(rename = "@table:table-background")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_background: Option<Boolean>,
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    #[serde(rename = "@draw:text-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawSizeAttlist {
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawStyleNameAttlist {
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:class-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_names: Option<StyleNameRefs>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawTextStyleNameAttlist {
    #[serde(rename = "@draw:text-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawTransformAttlist {
    #[serde(rename = "@draw:transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDrawViewboxAttlist {
    #[serde(rename = "@svg:viewBox")]
    pub view_box: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDrawZIndexAttlist {
    #[serde(rename = "@draw:z-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_index: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonDurTimingAttlist {
    #[serde(rename = "@smil:dur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dur: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonEditableAttlist {
    #[serde(rename = "@style:editable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonEndsyncTimingAttlist {
    #[serde(rename = "@smil:endsync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endsync: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDataStyleNameAttlist {
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDatabaseName {
    #[serde(rename = "@text:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "connection-resource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_resource: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonFieldDatabaseTable {
    #[serde(rename = "@text:table-name")]
    pub table_name: String,
    #[serde(rename = "@text:table-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_type: Option<String>,
    #[serde(rename = "@text:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "connection-resource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_resource: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonFieldDatabaseTableAttlist {
    #[serde(rename = "@text:table-name")]
    pub table_name: String,
    #[serde(rename = "@text:table-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDescriptionAttlist {
    #[serde(rename = "@text:description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDisplayValueFormulaAttlist {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDisplayValueFormulaNoneAttlist {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldDisplayValueNoneAttlist {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldFixedAttlist {
    #[serde(rename = "@text:fixed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldFormulaAttlist {
    #[serde(rename = "@text:formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonFieldNameAttlist {
    #[serde(rename = "@text:name")]
    pub name: VariableName,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFieldNumFormatAttlist {
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFillDefaultAttlist {
    #[serde(rename = "@smil:fillDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_default: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFillTimingAttlist {
    #[serde(rename = "@smil:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFormControlAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFormControlContent {
    #[serde(rename = "properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<String>,
    #[serde(rename = "event-listeners")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_listeners: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFormRelativeImagePositionAttlist {
    #[serde(rename = "@form:image-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_position: Option<String>,
    #[serde(rename = "@form:image-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_align: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFormVisualEffectAttlist {
    #[serde(rename = "@form:visual-effect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_effect: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonFormatSourceAttlist {
    #[serde(rename = "@number:format-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_source: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonHorizontalMarginAttlist {
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonInContentMetaAttlist {
    #[serde(rename = "@xhtml:about")]
    pub about: URIorSafeCURIE,
    #[serde(rename = "@xhtml:property")]
    pub property: CURIEs,
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonKeepWithNextAttlist {
    #[serde(rename = "@fo:keep-with-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonLinkedCell {
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonMarginAttlist {
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonMaxlengthAttlist {
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonMetaLiteralAttlist {
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonNumFormatAttlist {
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonNumFormatPrefixSuffixAttlist {
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonNumberAttlist {
    #[serde(rename = "@number:min-integer-digits")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_integer_digits: Option<Integer>,
    #[serde(rename = "@number:grouping")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonNumericControlAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonOfficeAnnotationNameAttlist {
    #[serde(rename = "@office:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonPaddingAttlist {
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonPageNumberAttlist {
    #[serde(rename = "@style:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonPresentationEffectAttlist {
    #[serde(rename = "@draw:shape-id")]
    pub shape_id: IDREF,
    #[serde(rename = "@presentation:effect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<PresentationEffects>,
    #[serde(rename = "@presentation:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<PresentationEffectDirections>,
    #[serde(rename = "@presentation:speed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PresentationSpeeds>,
    #[serde(rename = "@presentation:delay")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<Duration>,
    #[serde(rename = "@presentation:start-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_scale: Option<Percent>,
    #[serde(rename = "@presentation:path-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonPresentationHeaderFooterAttlist {
    #[serde(rename = "@presentation:use-header-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_header_name: Option<String>,
    #[serde(rename = "@presentation:use-footer-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_footer_name: Option<String>,
    #[serde(rename = "@presentation:use-date-time-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_date_time_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonPrintableAttlist {
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonReadonlyAttlist {
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonRepeat {
    #[serde(rename = "@form:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonRepeatTimingAttlist {
    #[serde(rename = "@smil:repeatDur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_dur: Option<String>,
    #[serde(rename = "@smil:repeatCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonRestartDefaultAttlist {
    #[serde(rename = "@smil:restartDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_default: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonRestartTimingAttlist {
    #[serde(rename = "@smil:restart")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonRotationAngleAttlist {
    #[serde(rename = "@style:rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_angle: Option<Angle>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonSectionAttlist {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:name")]
    pub name: String,
    #[serde(rename = "@text:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    #[serde(rename = "@text:protection-key")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key: Option<String>,
    #[serde(rename = "@text:protection-key-digest-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key_digest_algorithm: Option<AnyIRI>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonShadowAttlist {
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonSourceCellRange {
    #[serde(rename = "@form:source-cell-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cell_range: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonSpinButton {
    #[serde(rename = "@form:spin-button")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spin_button: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonSplineAnimValueAttlist {
    #[serde(rename = "@smil:keyTimes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_times: Option<String>,
    #[serde(rename = "@smil:keySplines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_splines: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonStockMarkerAttlist {
    #[serde(rename = "@chart:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonStyleDirectionAttlist {
    #[serde(rename = "@style:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonStyleHeaderFooterAttlist {
    #[serde(rename = "@style:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonSvgFontFaceXlinkAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonSvgGradientAttlist {
    #[serde(rename = "@svg:gradientUnits")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_units: Option<String>,
    #[serde(rename = "@svg:gradientTransform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_transform: Option<String>,
    #[serde(rename = "@svg:spreadMethod")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread_method: Option<String>,
    #[serde(rename = "@draw:name")]
    pub name: StyleName,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTabAttlist {
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTableCellAddressAttlist {
    #[serde(rename = "@table:column")]
    pub column: Integer,
    #[serde(rename = "@table:row")]
    pub row: Integer,
    #[serde(rename = "@table:table")]
    pub table: Integer,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTableCellRangeAddressAttlist {
    #[serde(rename = "@table:start-column")]
    pub start_column: Integer,
    #[serde(rename = "@table:start-row")]
    pub start_row: Integer,
    #[serde(rename = "@table:start-table")]
    pub start_table: Integer,
    #[serde(rename = "@table:end-column")]
    pub end_column: Integer,
    #[serde(rename = "@table:end-row")]
    pub end_row: Integer,
    #[serde(rename = "@table:end-table")]
    pub end_table: Integer,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTableChangeAttlist {
    #[serde(rename = "@table:id")]
    pub id: String,
    #[serde(rename = "@table:acceptance-state")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_state: Option<String>,
    #[serde(rename = "@table:rejecting-change-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejecting_change_id: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTableRangeAttlist {
    #[serde(rename = "@table:column")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<Integer>,
    #[serde(rename = "@table:row")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<Integer>,
    #[serde(rename = "@table:table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<Integer>,
    #[serde(rename = "@table:start-column")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_column: Option<Integer>,
    #[serde(rename = "@table:start-row")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_row: Option<Integer>,
    #[serde(rename = "@table:start-table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_table: Option<Integer>,
    #[serde(rename = "@table:end-column")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<Integer>,
    #[serde(rename = "@table:end-row")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_row: Option<Integer>,
    #[serde(rename = "@table:end-table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_table: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTableTemplateAttlist {
    #[serde(rename = "@table:style-name")]
    pub style_name: StyleNameRef,
    #[serde(rename = "@table:paragraph-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTextAlign {
    #[serde(rename = "@fo:text-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTextAnchorAttlist {
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTextSpreadsheetShapeAttlist {
    #[serde(rename = "@table:end-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_cell_address: Option<CellAddress>,
    #[serde(rename = "@table:end-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_x: Option<Coordinate>,
    #[serde(rename = "@table:end-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_y: Option<Coordinate>,
    #[serde(rename = "@table:table-background")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_background: Option<Boolean>,
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTimeManipAttlist {
    #[serde(rename = "@smil:accelerate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerate: Option<ZeroToOneDecimal>,
    #[serde(rename = "@smil:decelerate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decelerate: Option<ZeroToOneDecimal>,
    #[serde(rename = "@smil:autoReverse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_reverse: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTimingAttlist {
    #[serde(rename = "@smil:begin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    #[serde(rename = "@smil:end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(rename = "@smil:dur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dur: Option<String>,
    #[serde(rename = "@smil:repeatDur")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_dur: Option<String>,
    #[serde(rename = "@smil:repeatCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<String>,
    #[serde(rename = "@smil:restart")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(rename = "@smil:restartDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_default: Option<String>,
    #[serde(rename = "@smil:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(rename = "@smil:fillDefault")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_default: Option<String>,
    #[serde(rename = "@smil:accelerate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerate: Option<ZeroToOneDecimal>,
    #[serde(rename = "@smil:decelerate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decelerate: Option<ZeroToOneDecimal>,
    #[serde(rename = "@smil:autoReverse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_reverse: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonTitleAttlist {
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonValueAndTypeAttlist {
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "@office:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@office:currency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "@office:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@office:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<Duration>,
    #[serde(rename = "@office:boolean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<Boolean>,
    #[serde(rename = "@office:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonValueAttlist {
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonValueTypeAttlist {
    #[serde(rename = "@office:value-type")]
    pub value_type: ValueType,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonVerticalMarginAttlist {
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonVerticalPosAttlist {
    #[serde(rename = "@style:vertical-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_pos: Option<String>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonVerticalRelAttlist {
    #[serde(rename = "@style:vertical-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_rel: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonWritingModeAttlist {
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ConfigConfigItem = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConfigItemAttlist {
    #[serde(rename = "@config:name")]
    pub name: String,
    #[serde(rename = "@config:type")]
    pub r#type: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ConfigConfigItemMapEntry = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigConfigItemMapEntryAttlist {
    #[serde(rename = "@config:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ConfigConfigItemMapIndexed = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConfigItemMapIndexedAttlist {
    #[serde(rename = "@config:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ConfigConfigItemMapNamed = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConfigItemMapNamedAttlist {
    #[serde(rename = "@config:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type ConfigConfigItemSet = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConfigItemSetAttlist {
    #[serde(rename = "@config:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigItems {
    #[serde(rename = "config-item")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_item: Vec<String>,
    #[serde(rename = "config-item-set")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_item_set: Vec<String>,
    #[serde(rename = "config-item-map-named")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_item_map_named: Vec<String>,
    #[serde(rename = "config-item-map-indexed")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_item_map_indexed: Vec<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Controls {
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "textarea")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea: Option<String>,
    #[serde(rename = "formatted-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_text: Option<String>,
    #[serde(rename = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(rename = "combobox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combobox: Option<String>,
    #[serde(rename = "listbox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listbox: Option<String>,
    #[serde(rename = "checkbox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkbox: Option<String>,
    #[serde(rename = "password")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(rename = "fixed-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_text: Option<String>,
    #[serde(rename = "button")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
    #[serde(rename = "image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(rename = "radio")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "image-frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_frame: Option<String>,
    #[serde(rename = "hidden")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<String>,
    #[serde(rename = "grid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<String>,
    #[serde(rename = "value-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_range: Option<String>,
    #[serde(rename = "generic-control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generic_control: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencySymbolAndText {
    #[serde(rename = "currency-symbol")]
    pub currency_symbol: String,
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "fill-character")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_character: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentSelected {
    #[serde(rename = "@form:current-selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_selected: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomShapeType {
    #[serde(rename = "$text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DateOrDateTime;

pub type DbApplicationConnectionSettings = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbApplicationConnectionSettingsAttlist {
    #[serde(rename = "@db:is-table-name-length-limited")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_table_name_length_limited: Option<Boolean>,
    #[serde(rename = "@db:enable-sql92-check")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_sql92_check: Option<Boolean>,
    #[serde(rename = "@db:append-table-alias-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_table_alias_name: Option<Boolean>,
    #[serde(rename = "@db:ignore-driver-privileges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_driver_privileges: Option<Boolean>,
    #[serde(rename = "@db:boolean-comparison-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_comparison_mode: Option<String>,
    #[serde(rename = "@db:use-catalog")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_catalog: Option<Boolean>,
    #[serde(rename = "@db:max-row-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_row_count: Option<Integer>,
    #[serde(rename = "@db:suppress-version-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_version_columns: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbApplyCommand {
    #[serde(rename = "@db:apply-command")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_command: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbAutoIncrement = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbAutoIncrementAttlist {
    #[serde(rename = "@db:additional-column-statement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_column_statement: Option<String>,
    #[serde(rename = "@db:row-retrieving-statement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_retrieving_statement: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbCharacterSet = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbCharacterSetAttlist {
    #[serde(rename = "@db:encoding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<TextEncoding>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbColumn = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbColumnAttlist {
    #[serde(rename = "@db:visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<Boolean>,
    #[serde(rename = "@db:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@db:default-cell-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_cell_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbColumnDefinition = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbColumnDefinitionAttlist {
    #[serde(rename = "@db:name")]
    pub name: String,
    #[serde(rename = "@db:data-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DbDataTypes>,
    #[serde(rename = "@db:type-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    #[serde(rename = "@db:precision")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<PositiveInteger>,
    #[serde(rename = "@db:scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<PositiveInteger>,
    #[serde(rename = "@db:is-nullable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_nullable: Option<String>,
    #[serde(rename = "@db:is-empty-allowed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_empty_allowed: Option<Boolean>,
    #[serde(rename = "@db:is-autoincrement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_autoincrement: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbColumnDefinitions = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbColumnDefinitionsAttlist;

pub type DbColumns = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbColumnsAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCommand {
    #[serde(rename = "@db:command")]
    pub command: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbComponent = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbComponentAttlist {
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@db:as-template")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_template: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbComponentCollection = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbComponentCollectionAttlist;

pub type DbConnectionData = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbConnectionDataAttlist;

pub type DbConnectionResource = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionResourceAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbDataSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDataSourceAttlist;

pub type DbDataSourceSetting = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbDataSourceSettingAttlist {
    #[serde(rename = "@db:data-source-setting-is-list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source_setting_is_list: Option<Boolean>,
    #[serde(rename = "@db:data-source-setting-name")]
    pub data_source_setting_name: String,
    #[serde(rename = "@db:data-source-setting-type")]
    pub data_source_setting_type: DbDataSourceSettingTypes,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbDataSourceSettingValue = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDataSourceSettingValueAttlist;

pub type DbDataSourceSettings = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDataSourceSettingsAttlist;

pub type DbDatabaseDescription = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDatabaseDescriptionAttlist;

pub type DbDelimiter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDelimiterAttlist {
    #[serde(rename = "@db:field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "@db:string")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string: Option<String>,
    #[serde(rename = "@db:decimal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal: Option<String>,
    #[serde(rename = "@db:thousand")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thousand: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbDriverSettings = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDriverSettingsAttlist {
    #[serde(rename = "@db:show-deleted")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_deleted: Option<Boolean>,
    #[serde(rename = "@db:system-driver-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_driver_settings: Option<String>,
    #[serde(rename = "@db:base-dn")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_dn: Option<String>,
    #[serde(rename = "@db:is-first-row-header-line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_first_row_header_line: Option<Boolean>,
    #[serde(rename = "@db:parameter-name-substitution")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_name_substitution: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbFileBasedDatabase = DbFileBasedDatabaseAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbFileBasedDatabaseAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@db:media-type")]
    pub media_type: String,
    #[serde(rename = "@db:extension")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbFilterStatement = String;

pub type DbForms = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbFormsAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbHostAndPort {
    #[serde(rename = "@db:hostname")]
    pub hostname: String,
    #[serde(rename = "@db:port")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbIndex = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbIndexAttlist {
    #[serde(rename = "@db:name")]
    pub name: String,
    #[serde(rename = "@db:catalog-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(rename = "@db:is-unique")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_unique: Option<Boolean>,
    #[serde(rename = "@db:is-clustered")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_clustered: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbIndexColumn = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbIndexColumnAttlist {
    #[serde(rename = "@db:name")]
    pub name: String,
    #[serde(rename = "@db:is-ascending")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_ascending: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbIndexColumns = String;

pub type DbIndices = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbIndicesAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbIsFirstRowHeaderLine {
    #[serde(rename = "@db:is-first-row-header-line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_first_row_header_line: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbKey = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbKeyAttlist {
    #[serde(rename = "@db:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@db:type")]
    pub r#type: String,
    #[serde(rename = "@db:referenced-table-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referenced_table_name: Option<String>,
    #[serde(rename = "@db:update-rule")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_rule: Option<String>,
    #[serde(rename = "@db:delete-rule")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_rule: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbKeyColumn = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbKeyColumnAttlist {
    #[serde(rename = "@db:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@db:related-column-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_column_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbKeyColumns = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbKeyColumnsAttlist;

pub type DbKeys = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbKeysAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbLocalSocketName {
    #[serde(rename = "@db:local-socket")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_socket: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbLogin = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbLoginAttlist {
    #[serde(rename = "@db:user-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(rename = "@db:use-system-user")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_system_user: Option<Boolean>,
    #[serde(rename = "@db:is-password-required")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_password_required: Option<Boolean>,
    #[serde(rename = "@db:login-timeout")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_timeout: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbOrderStatement = String;

pub type DbQueries = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbQueriesAttlist;

pub type DbQuery = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbQueryAttlist {
    #[serde(rename = "@db:command")]
    pub command: String,
    #[serde(rename = "@db:escape-processing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escape_processing: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbQueryCollection = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbQueryCollectionAttlist;

pub type DbReports = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbReportsAttlist;

pub type DbSchemaDefinition = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbSchemaDefinitionAttlist;

pub type DbServerDatabase = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbServerDatabaseAttlist {
    #[serde(rename = "@db:type")]
    pub r#type: NamespacedToken,
    #[serde(rename = "@db:hostname")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(rename = "@db:port")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<PositiveInteger>,
    #[serde(rename = "@db:local-socket")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_socket: Option<String>,
    #[serde(rename = "@db:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbShowDeleted {
    #[serde(rename = "@db:show-deleted")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_deleted: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbTableDefinition = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableDefinitionAttlist {
    #[serde(rename = "@db:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbTableDefinitions = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableDefinitionsAttlist;

pub type DbTableExcludeFilter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableExcludeFilterAttlist;

pub type DbTableFilter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableFilterAttlist;

pub type DbTableFilterPattern = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableFilterPatternAttlist;

pub type DbTableIncludeFilter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableIncludeFilterAttlist;

pub type DbTablePresentation = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTablePresentationAttlist;

pub type DbTablePresentations = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTablePresentationsAttlist;

pub type DbTableSetting = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableSettingAttlist {
    #[serde(rename = "@db:is-first-row-header-line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_first_row_header_line: Option<Boolean>,
    #[serde(rename = "@db:show-deleted")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_deleted: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DbTableSettings = String;

pub type DbTableType = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableTypeAttlist;

pub type DbTableTypeFilter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableTypeFilterAttlist;

pub type DbUpdateTable = CommonDbTableNameAttlist;

pub type DcCreator = String;

pub type DcDate = DateTime;

pub type Dr3dCube = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dr3dCubeAttlist {
    #[serde(rename = "@dr3d:min-edge")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_edge: Option<Vector3D>,
    #[serde(rename = "@dr3d:max-edge")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_edge: Option<Vector3D>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type Dr3dExtrude = String;

pub type Dr3dLight = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dr3dLightAttlist {
    #[serde(rename = "@dr3d:diffuse-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffuse_color: Option<Color>,
    #[serde(rename = "@dr3d:direction")]
    pub direction: Vector3D,
    #[serde(rename = "@dr3d:enabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Boolean>,
    #[serde(rename = "@dr3d:specular")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specular: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type Dr3dRotate = String;

pub type Dr3dScene = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dr3dSceneAttlist {
    #[serde(rename = "@dr3d:vrp")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vrp: Option<Vector3D>,
    #[serde(rename = "@dr3d:vpn")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn: Option<Vector3D>,
    #[serde(rename = "@dr3d:vup")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vup: Option<Vector3D>,
    #[serde(rename = "@dr3d:projection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
    #[serde(rename = "@dr3d:distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<Length>,
    #[serde(rename = "@dr3d:focal-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<Length>,
    #[serde(rename = "@dr3d:shadow-slant")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_slant: Option<Angle>,
    #[serde(rename = "@dr3d:shade-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shade_mode: Option<String>,
    #[serde(rename = "@dr3d:ambient-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<Color>,
    #[serde(rename = "@dr3d:lighting-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting_mode: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type Dr3dSphere = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dr3dSphereAttlist {
    #[serde(rename = "@dr3d:center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center: Option<Vector3D>,
    #[serde(rename = "@dr3d:size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Vector3D>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawA = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawAAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@office:target-frame-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame_name: Option<TargetFrameName>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@office:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@office:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@office:server-map")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_map: Option<Boolean>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawApplet = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawAppletAttlist {
    #[serde(rename = "@draw:code")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(rename = "@draw:object")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(rename = "@draw:archive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
    #[serde(rename = "@draw:may-script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub may_script: Option<Boolean>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawAreaCircle = String;

pub type DrawAreaPolygon = String;

pub type DrawAreaRectangle = String;

pub type DrawCaption = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawCaptionAttlist {
    #[serde(rename = "@draw:caption-point-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_point_x: Option<Coordinate>,
    #[serde(rename = "@draw:caption-point-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_point_y: Option<Coordinate>,
    #[serde(rename = "@draw:corner-radius")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<NonNegativeLength>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawCircle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCircleAttlist {
    #[serde(rename = "@svg:r")]
    pub r: Length,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawConnector = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawConnectorAttlist {
    #[serde(rename = "@draw:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@svg:x1")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x1: Option<Coordinate>,
    #[serde(rename = "@svg:y1")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y1: Option<Coordinate>,
    #[serde(rename = "@draw:start-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_shape: Option<IDREF>,
    #[serde(rename = "@draw:start-glue-point")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_glue_point: Option<NonNegativeInteger>,
    #[serde(rename = "@svg:x2")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x2: Option<Coordinate>,
    #[serde(rename = "@svg:y2")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<Coordinate>,
    #[serde(rename = "@draw:end-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_shape: Option<IDREF>,
    #[serde(rename = "@draw:end-glue-point")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_glue_point: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:line-skew")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_skew: Option<String>,
    #[serde(rename = "@svg:d")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<PathData>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawContourPath = String;

pub type DrawContourPolygon = String;

pub type DrawControl = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawControlAttlist {
    #[serde(rename = "@draw:control")]
    pub control: IDREF,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawCustomShape = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawCustomShapeAttlist {
    #[serde(rename = "@draw:engine")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<NamespacedToken>,
    #[serde(rename = "@draw:data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawEllipse = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawEllipseAttlist {
    #[serde(rename = "@svg:rx")]
    pub rx: Length,
    #[serde(rename = "@svg:ry")]
    pub ry: Length,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawEnhancedGeometry = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawEnhancedGeometryAttlist {
    #[serde(rename = "@draw:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CustomShapeType>,
    #[serde(rename = "@svg:viewBox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_box: Option<String>,
    #[serde(rename = "@draw:mirror-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_vertical: Option<Boolean>,
    #[serde(rename = "@draw:mirror-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_horizontal: Option<Boolean>,
    #[serde(rename = "@draw:text-rotate-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotate_angle: Option<Angle>,
    #[serde(rename = "@draw:extrusion-allowed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_allowed: Option<Boolean>,
    #[serde(rename = "@draw:text-path-allowed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path_allowed: Option<Boolean>,
    #[serde(rename = "@draw:concentric-gradient-fill-allowed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concentric_gradient_fill_allowed: Option<Boolean>,
    #[serde(rename = "@draw:extrusion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion: Option<Boolean>,
    #[serde(rename = "@draw:extrusion-brightness")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_brightness: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:extrusion-depth")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_depth: Option<String>,
    #[serde(rename = "@draw:extrusion-diffusion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_diffusion: Option<Percent>,
    #[serde(rename = "@draw:extrusion-number-of-line-segments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_number_of_line_segments: Option<Integer>,
    #[serde(rename = "@draw:extrusion-light-face")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_light_face: Option<Boolean>,
    #[serde(rename = "@draw:extrusion-first-light-harsh")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_first_light_harsh: Option<Boolean>,
    #[serde(rename = "@draw:extrusion-second-light-harsh")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_second_light_harsh: Option<Boolean>,
    #[serde(rename = "@draw:extrusion-first-light-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_first_light_level: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:extrusion-second-light-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_second_light_level: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:extrusion-first-light-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_first_light_direction: Option<Vector3D>,
    #[serde(rename = "@draw:extrusion-second-light-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_second_light_direction: Option<Vector3D>,
    #[serde(rename = "@draw:extrusion-metal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_metal: Option<Boolean>,
    #[serde(rename = "@dr3d:shade-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shade_mode: Option<String>,
    #[serde(rename = "@draw:extrusion-rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_rotation_angle: Option<String>,
    #[serde(rename = "@draw:extrusion-rotation-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_rotation_center: Option<Vector3D>,
    #[serde(rename = "@draw:extrusion-shininess")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_shininess: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:extrusion-skew")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_skew: Option<String>,
    #[serde(rename = "@draw:extrusion-specularity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_specularity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@dr3d:projection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
    #[serde(rename = "@draw:extrusion-viewpoint")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_viewpoint: Option<Point3D>,
    #[serde(rename = "@draw:extrusion-origin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_origin: Option<String>,
    #[serde(rename = "@draw:extrusion-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrusion_color: Option<Boolean>,
    #[serde(rename = "@draw:enhanced-path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enhanced_path: Option<String>,
    #[serde(rename = "@draw:path-stretchpoint-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_stretchpoint_x: Option<Double>,
    #[serde(rename = "@draw:path-stretchpoint-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_stretchpoint_y: Option<Double>,
    #[serde(rename = "@draw:text-areas")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_areas: Option<String>,
    #[serde(rename = "@draw:glue-points")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glue_points: Option<String>,
    #[serde(rename = "@draw:glue-point-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glue_point_type: Option<String>,
    #[serde(rename = "@draw:glue-point-leaving-directions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glue_point_leaving_directions: Option<String>,
    #[serde(rename = "@draw:text-path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path: Option<Boolean>,
    #[serde(rename = "@draw:text-path-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path_mode: Option<String>,
    #[serde(rename = "@draw:text-path-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path_scale: Option<String>,
    #[serde(rename = "@draw:text-path-same-letter-heights")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path_same_letter_heights: Option<Boolean>,
    #[serde(rename = "@draw:modifiers")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawEquation = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawEquationAttlist {
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@draw:formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawFillImage = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawFillImageAttlist {
    #[serde(rename = "@draw:name")]
    pub name: StyleName,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawFloatingFrame = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawFloatingFrameAttlist {
    #[serde(rename = "@draw:frame-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_name: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawFrame = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawFrameAttlist {
    #[serde(rename = "@draw:copy-of")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_of: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawG = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawGAttlist {
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawGluePoint = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawGluePointAttlist {
    #[serde(rename = "@draw:id")]
    pub id: NonNegativeInteger,
    #[serde(rename = "@svg:x")]
    pub x: String,
    #[serde(rename = "@svg:y")]
    pub y: String,
    #[serde(rename = "@draw:align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(rename = "@draw:escape-direction")]
    pub escape_direction: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawGradient = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawGradientAttlist {
    #[serde(rename = "@draw:start-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_color: Option<Color>,
    #[serde(rename = "@draw:end-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_color: Option<Color>,
    #[serde(rename = "@draw:start-intensity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_intensity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:end-intensity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_intensity: Option<ZeroToHundredPercent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawHandle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawHandleAttlist {
    #[serde(rename = "@draw:handle-mirror-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_mirror_vertical: Option<Boolean>,
    #[serde(rename = "@draw:handle-mirror-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_mirror_horizontal: Option<Boolean>,
    #[serde(rename = "@draw:handle-switched")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_switched: Option<Boolean>,
    #[serde(rename = "@draw:handle-position")]
    pub handle_position: String,
    #[serde(rename = "@draw:handle-range-x-minimum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_range_x_minimum: Option<String>,
    #[serde(rename = "@draw:handle-range-x-maximum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_range_x_maximum: Option<String>,
    #[serde(rename = "@draw:handle-range-y-minimum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_range_y_minimum: Option<String>,
    #[serde(rename = "@draw:handle-range-y-maximum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_range_y_maximum: Option<String>,
    #[serde(rename = "@draw:handle-polar")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_polar: Option<String>,
    #[serde(rename = "@draw:handle-radius-range-minimum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_radius_range_minimum: Option<String>,
    #[serde(rename = "@draw:handle-radius-range-maximum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_radius_range_maximum: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawHatch = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawHatchAttlist {
    #[serde(rename = "@draw:name")]
    pub name: StyleName,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@draw:style")]
    pub style: String,
    #[serde(rename = "@draw:color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(rename = "@draw:distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<Length>,
    #[serde(rename = "@draw:rotation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Angle>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawImage = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawImageAttlist {
    #[serde(rename = "@draw:filter-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_name: Option<String>,
    #[serde(rename = "@draw:mime-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawImageMap = String;

pub type DrawLayer = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawLayerAttlist {
    #[serde(rename = "@draw:name")]
    pub name: String,
    #[serde(rename = "@draw:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    #[serde(rename = "@draw:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawLayerSet = String;

pub type DrawLine = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawLineAttlist {
    #[serde(rename = "@svg:x1")]
    pub x1: Coordinate,
    #[serde(rename = "@svg:y1")]
    pub y1: Coordinate,
    #[serde(rename = "@svg:x2")]
    pub x2: Coordinate,
    #[serde(rename = "@svg:y2")]
    pub y2: Coordinate,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawMarker = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawMarkerAttlist {
    #[serde(rename = "@draw:name")]
    pub name: StyleName,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawMeasure = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawMeasureAttlist {
    #[serde(rename = "@svg:x1")]
    pub x1: Coordinate,
    #[serde(rename = "@svg:y1")]
    pub y1: Coordinate,
    #[serde(rename = "@svg:x2")]
    pub x2: Coordinate,
    #[serde(rename = "@svg:y2")]
    pub y2: Coordinate,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawObject = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawObjectAttlist {
    #[serde(rename = "@draw:notify-on-update-of-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify_on_update_of_ranges: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawObjectOle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawObjectOleAttlist {
    #[serde(rename = "@draw:class-id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawOpacity = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawOpacityAttlist {
    #[serde(rename = "@draw:start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<ZeroToHundredPercent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawPage = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawPageAttlist {
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:master-page-name")]
    pub master_page_name: StyleNameRef,
    #[serde(rename = "@presentation:presentation-page-layout-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation_page_layout_name: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@draw:nav-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nav_order: Option<IDREFS>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawPageThumbnail = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawPageThumbnailAttlist {
    #[serde(rename = "@draw:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawParam = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawParamAttlist {
    #[serde(rename = "@draw:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@draw:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawPath = String;

pub type DrawPlugin = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawPluginAttlist {
    #[serde(rename = "@draw:mime-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawPolygon = String;

pub type DrawPolyline = String;

pub type DrawRect = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawRectAttlist {
    #[serde(rename = "@draw:corner-radius")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<NonNegativeLength>,
    #[serde(rename = "@svg:rx")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx: Option<NonNegativeLength>,
    #[serde(rename = "@svg:ry")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ry: Option<NonNegativeLength>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawRegularPolygon = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawRegularPolygonAttlist {
    #[serde(rename = "@draw:concave")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concave: Option<String>,
    #[serde(rename = "@draw:sharpness")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<Percent>,
    #[serde(rename = "@draw:corners")]
    pub corners: PositiveInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawRegularPolygonSharpnessAttlist {
    #[serde(rename = "@draw:sharpness")]
    pub sharpness: Percent,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type DrawStrokeDash = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawStrokeDashAttlist {
    #[serde(rename = "@draw:name")]
    pub name: StyleName,
    #[serde(rename = "@draw:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@draw:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(rename = "@draw:dots1")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dots1: Option<Integer>,
    #[serde(rename = "@draw:dots1-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dots1_length: Option<String>,
    #[serde(rename = "@draw:dots2")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dots2: Option<Integer>,
    #[serde(rename = "@draw:dots2-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dots2_length: Option<String>,
    #[serde(rename = "@draw:distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawText {
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub p: Vec<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type DrawTextBox = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawTextBoxAttlist {
    #[serde(rename = "@draw:chain-next-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_next_name: Option<String>,
    #[serde(rename = "@draw:corner-radius")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<NonNegativeLength>,
    #[serde(rename = "@fo:min-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<String>,
    #[serde(rename = "@fo:min-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,
    #[serde(rename = "@fo:max-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<String>,
    #[serde(rename = "@fo:max-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dropdown {
    #[serde(rename = "@form:dropdown")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropdown: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct For {
    #[serde(rename = "@form:for")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#for: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormButtonAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:button-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_type: Option<Types>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:image-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data: Option<AnyIRI>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@office:target-frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame: Option<TargetFrameName>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:image-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_position: Option<String>,
    #[serde(rename = "@form:image-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_align: Option<String>,
    #[serde(rename = "@form:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Boolean>,
    #[serde(rename = "@form:delay-for-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_for_repeat: Option<Duration>,
    #[serde(rename = "@form:default-button")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_button: Option<Boolean>,
    #[serde(rename = "@form:toggle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toggle: Option<Boolean>,
    #[serde(rename = "@form:focus-on-click")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_on_click: Option<Boolean>,
    #[serde(rename = "@form:xforms-submission")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xforms_submission: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormCheckboxAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:visual-effect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_effect: Option<String>,
    #[serde(rename = "@form:image-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_position: Option<String>,
    #[serde(rename = "@form:image-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_align: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:current-state")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_state: Option<States>,
    #[serde(rename = "@form:is-tristate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_tristate: Option<Boolean>,
    #[serde(rename = "@form:state")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<States>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormColumn = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormColumnAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:text-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormComboboxAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:dropdown")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropdown: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:list-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source: Option<String>,
    #[serde(rename = "@form:list-source-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source_type: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:source-cell-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cell_range: Option<String>,
    #[serde(rename = "@form:auto-complete")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_complete: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormConnectionResource = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormControlAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormDateAttlist {
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Date>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<Date>,
    #[serde(rename = "@form:min-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<Date>,
    #[serde(rename = "@form:max-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<Date>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFileAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFixedTextAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:for")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#for: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:multi-line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_line: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormForm = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormFormAttlist {
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@office:target-frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame: Option<TargetFrameName>,
    #[serde(rename = "@form:method")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(rename = "@form:enctype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enctype: Option<String>,
    #[serde(rename = "@form:allow-deletes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_deletes: Option<Boolean>,
    #[serde(rename = "@form:allow-inserts")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_inserts: Option<Boolean>,
    #[serde(rename = "@form:allow-updates")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_updates: Option<Boolean>,
    #[serde(rename = "@form:apply-filter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_filter: Option<Boolean>,
    #[serde(rename = "@form:command-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_type: Option<String>,
    #[serde(rename = "@form:command")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(rename = "@form:datasource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datasource: Option<String>,
    #[serde(rename = "@form:master-fields")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_fields: Option<String>,
    #[serde(rename = "@form:detail-fields")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_fields: Option<String>,
    #[serde(rename = "@form:escape-processing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escape_processing: Option<Boolean>,
    #[serde(rename = "@form:filter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "@form:ignore-result")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_result: Option<Boolean>,
    #[serde(rename = "@form:navigation-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation_mode: Option<Navigation>,
    #[serde(rename = "@form:order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    #[serde(rename = "@form:tab-cycle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_cycle: Option<TabCycles>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFormattedTextAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:spin-button")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spin_button: Option<Boolean>,
    #[serde(rename = "@form:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Boolean>,
    #[serde(rename = "@form:delay-for-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_for_repeat: Option<Duration>,
    #[serde(rename = "@form:max-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    #[serde(rename = "@form:min-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(rename = "@form:validation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFrameAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:for")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#for: Option<String>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormGenericControlAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormGridAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormHiddenAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormImageAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:button-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_type: Option<Types>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:image-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data: Option<AnyIRI>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@office:target-frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame: Option<TargetFrameName>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormImageFrameAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:image-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data: Option<AnyIRI>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormItem = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormItemAttlist {
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormListboxAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:dropdown")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropdown: Option<Boolean>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:bound-column")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_column: Option<String>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:list-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source: Option<String>,
    #[serde(rename = "@form:list-source-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source_type: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:list-linkage-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_linkage_type: Option<String>,
    #[serde(rename = "@form:source-cell-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cell_range: Option<String>,
    #[serde(rename = "@form:multiple")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple: Option<Boolean>,
    #[serde(rename = "@form:xforms-list-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xforms_list_source: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormNumberAttlist {
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<Double>,
    #[serde(rename = "@form:min-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<Double>,
    #[serde(rename = "@form:max-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<Double>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormOption = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormOptionAttlist {
    #[serde(rename = "@form:current-selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_selected: Option<Boolean>,
    #[serde(rename = "@form:selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<Boolean>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPasswordAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:echo-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo_char: Option<Character>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type FormProperties = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormProperty {
    #[serde(rename = "property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(rename = "list-property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_property: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPropertyName {
    #[serde(rename = "@form:property-name")]
    pub property_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormPropertyTypeAndValueList {
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "list-value")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list_value: Vec<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormPropertyValueAndTypeAttlist {
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "@office:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@office:currency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "@office:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@office:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<Duration>,
    #[serde(rename = "@office:boolean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<Boolean>,
    #[serde(rename = "@office:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormRadioAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_selected: Option<Boolean>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:visual-effect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_effect: Option<String>,
    #[serde(rename = "@form:image-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_position: Option<String>,
    #[serde(rename = "@form:image-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_align: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormTextAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormTextareaAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:max-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<NonNegativeInteger>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:readonly")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:convert-empty-to-null")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_empty_to_null: Option<Boolean>,
    #[serde(rename = "@form:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormTimeAttlist {
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Time>,
    #[serde(rename = "@form:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<Time>,
    #[serde(rename = "@form:min-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<Time>,
    #[serde(rename = "@form:max-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<Time>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormValueRangeAttlist {
    #[serde(rename = "@form:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@form:control-implementation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_implementation: Option<NamespacedToken>,
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(rename = "@form:disabled")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Boolean>,
    #[serde(rename = "@form:printable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printable: Option<Boolean>,
    #[serde(rename = "@form:tab-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<NonNegativeInteger>,
    #[serde(rename = "@form:tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop: Option<Boolean>,
    #[serde(rename = "@form:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@form:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "@form:linked-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_cell: Option<String>,
    #[serde(rename = "@form:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Boolean>,
    #[serde(rename = "@form:delay-for-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_for_repeat: Option<Duration>,
    #[serde(rename = "@form:max-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<Integer>,
    #[serde(rename = "@form:min-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<Integer>,
    #[serde(rename = "@form:step-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_size: Option<PositiveInteger>,
    #[serde(rename = "@form:page-step-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_step_size: Option<PositiveInteger>,
    #[serde(rename = "@form:orientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeaderFooterContent {
    #[serde(rename = "tracked-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_changes: Option<String>,
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub h: Vec<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub p: Vec<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<String>,
    #[serde(rename = "table")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<String>,
    #[serde(rename = "section")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub section: Vec<String>,
    #[serde(rename = "table-of-content")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_of_content: Vec<String>,
    #[serde(rename = "illustration-index")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub illustration_index: Vec<String>,
    #[serde(rename = "table-index")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_index: Vec<String>,
    #[serde(rename = "object-index")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_index: Vec<String>,
    #[serde(rename = "user-index")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_index: Vec<String>,
    #[serde(rename = "alphabetical-index")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alphabetical_index: Vec<String>,
    #[serde(rename = "bibliography")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bibliography: Vec<String>,
    #[serde(rename = "index-title")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_title: Vec<String>,
    #[serde(rename = "region-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_left: Option<RegionContent>,
    #[serde(rename = "region-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_center: Option<RegionContent>,
    #[serde(rename = "region-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_right: Option<RegionContent>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingAttrs {
    #[serde(rename = "@text:outline-level")]
    pub outline_level: PositiveInteger,
    #[serde(rename = "@text:restart-numbering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_numbering: Option<Boolean>,
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<NonNegativeInteger>,
    #[serde(rename = "@text:is-list-header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_list_header: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageData {
    #[serde(rename = "@form:image-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data: Option<AnyIRI>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexContentMain {
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    #[serde(rename = "numbered-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_paragraph: Option<String>,
    #[serde(rename = "table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "section")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-of-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_of_content: Option<String>,
    #[serde(rename = "illustration-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration_index: Option<String>,
    #[serde(rename = "table-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_index: Option<String>,
    #[serde(rename = "object-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<String>,
    #[serde(rename = "user-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index: Option<String>,
    #[serde(rename = "alphabetical-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index: Option<String>,
    #[serde(rename = "bibliography")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<String>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    #[serde(rename = "index-title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_title: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Label {
    #[serde(rename = "@form:label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineWidth {
    #[serde(rename = "$text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListLinkageType {
    #[serde(rename = "@form:list-linkage-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_linkage_type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListSource {
    #[serde(rename = "@form:list-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListSourceType {
    #[serde(rename = "@form:list-source-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_source_type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type MathMath = MathMarkup;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MathMarkup {
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type MetaDateString = String;

pub type NumberAmPm = ();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberAndText {
    #[serde(rename = "number")]
    pub number: String,
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "fill-character")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_character: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type NumberBoolean = ();

pub type NumberBooleanStyle = String;

pub type NumberCurrencyStyle = String;

pub type NumberCurrencySymbol = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberCurrencySymbolAttlist {
    #[serde(rename = "@number:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@number:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@number:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@number:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberDateStyle = String;

pub type NumberDay = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberDayAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberDayOfWeek = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberDayOfWeekAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberEmbeddedText = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberEmbeddedTextAttlist {
    #[serde(rename = "@number:position")]
    pub position: Integer,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberEra = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberEraAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberFillCharacter = String;

pub type NumberFraction = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberFractionAttlist {
    #[serde(rename = "@number:min-numerator-digits")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_numerator_digits: Option<Integer>,
    #[serde(rename = "@number:min-denominator-digits")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_denominator_digits: Option<Integer>,
    #[serde(rename = "@number:denominator-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denominator_value: Option<Integer>,
    #[serde(rename = "@number:max-denominator-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_denominator_value: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberHours = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberHoursAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberMinutes = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberMinutesAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberMonth = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberMonthAttlist {
    #[serde(rename = "@number:textual")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textual: Option<Boolean>,
    #[serde(rename = "@number:possessive-form")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub possessive_form: Option<Boolean>,
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberNumber = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberNumberAttlist {
    #[serde(rename = "@number:decimal-replacement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_replacement: Option<String>,
    #[serde(rename = "@number:display-factor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_factor: Option<Double>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberNumberStyle = String;

pub type NumberPercentageStyle = String;

pub type NumberQuarter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberQuarterAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberScientificNumber = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberScientificNumberAttlist {
    #[serde(rename = "@number:min-exponent-digits")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_exponent_digits: Option<Integer>,
    #[serde(rename = "@number:exponent-interval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exponent_interval: Option<PositiveInteger>,
    #[serde(rename = "@number:forced-exponent-sign")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forced_exponent_sign: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberSeconds = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberSecondsAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(rename = "@number:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberText = String;

pub type NumberTextContent = ();

pub type NumberTextStyle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberTextWithFillchar {
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "fill-character")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_character: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type NumberTimeStyle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberTimeStyleAttlist {
    #[serde(rename = "@number:truncate-on-overflow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncate_on_overflow: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type NumberWeekOfYear = String;

pub type NumberYear = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumberYearAttlist {
    #[serde(rename = "@number:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type OfficeAnnotation = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeAnnotationAttlist {
    #[serde(rename = "@office:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    #[serde(rename = "@office:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type OfficeAnnotationEnd = OfficeAnnotationEndAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeAnnotationEndAttlist {
    #[serde(rename = "@office:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeAutomaticStyles {
    #[serde(rename = "automatic-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_styles: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type OfficeBinaryData = Base64Binary;

pub type OfficeBody = OfficeBodyContent;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeBodyContent {
    #[serde(rename = "text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "drawing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<String>,
    #[serde(rename = "presentation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<String>,
    #[serde(rename = "spreadsheet")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spreadsheet: Option<String>,
    #[serde(rename = "chart")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart: Option<String>,
    #[serde(rename = "image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(rename = "database")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type OfficeChangeInfo = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeChartAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeChartContentEpilogue {
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeChartContentMain {
    #[serde(rename = "chart")]
    pub chart: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeChartContentPrelude {
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type OfficeDatabase = String;

pub type OfficeDdeSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeDdeSourceAttlist {
    #[serde(rename = "@office:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@office:conversion-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type OfficeDocument = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeDocumentAttrs {
    #[serde(rename = "@office:mimetype")]
    pub mimetype: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeDocumentCommonAttrs {
    #[serde(rename = "@office:version")]
    pub version: String,
    #[serde(rename = "@grddl:transformation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transformation: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type OfficeDocumentContent = String;

pub type OfficeDocumentMeta = String;

pub type OfficeDocumentSettings = String;

pub type OfficeDocumentStyles = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeDrawingAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeDrawingContentEpilogue {
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeDrawingContentMain {
    #[serde(rename = "page")]
    pub page: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeDrawingContentPrelude {
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type OfficeEventListeners = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeFontFaceDecls {
    #[serde(rename = "font-face-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_face_decls: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeForms {
    #[serde(rename = "forms")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forms: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeFormsAttlist {
    #[serde(rename = "@form:automatic-focus")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_focus: Option<Boolean>,
    #[serde(rename = "@form:apply-design-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_design_mode: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeImageAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeImageContentEpilogue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeImageContentMain {
    #[serde(rename = "frame")]
    pub frame: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeImageContentPrelude;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeMasterStyles {
    #[serde(rename = "master-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_styles: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeMeta {
    #[serde(rename = "meta")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<OfficeMetaContentStrict>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeMetaContentStrict {
    #[serde(rename = "generator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(rename = "title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "subject")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "keyword")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(rename = "initial-creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_creator: Option<String>,
    #[serde(rename = "creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(rename = "printed-by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_by: Option<String>,
    #[serde(rename = "creation-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<DateTime>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime>,
    #[serde(rename = "print-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_date: Option<DateTime>,
    #[serde(rename = "template")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(rename = "auto-reload")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_reload: Option<String>,
    #[serde(rename = "hyperlink-behaviour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperlink_behaviour: Option<String>,
    #[serde(rename = "language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(rename = "editing-cycles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_cycles: Option<NonNegativeInteger>,
    #[serde(rename = "editing-duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_duration: Option<Duration>,
    #[serde(rename = "document-statistic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_statistic: Option<String>,
    #[serde(rename = "user-defined")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeMetaData {
    #[serde(rename = "generator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(rename = "title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "subject")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "keyword")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(rename = "initial-creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_creator: Option<String>,
    #[serde(rename = "creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(rename = "printed-by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_by: Option<String>,
    #[serde(rename = "creation-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<DateTime>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime>,
    #[serde(rename = "print-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_date: Option<DateTime>,
    #[serde(rename = "template")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(rename = "auto-reload")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_reload: Option<String>,
    #[serde(rename = "hyperlink-behaviour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperlink_behaviour: Option<String>,
    #[serde(rename = "language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(rename = "editing-cycles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_cycles: Option<NonNegativeInteger>,
    #[serde(rename = "editing-duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_duration: Option<Duration>,
    #[serde(rename = "document-statistic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_statistic: Option<String>,
    #[serde(rename = "user-defined")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficePresentationAttlist;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficePresentationContentEpilogue {
    #[serde(rename = "settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficePresentationContentMain {
    #[serde(rename = "page")]
    pub page: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficePresentationContentPrelude {
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    #[serde(rename = "header-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_decl: Option<String>,
    #[serde(rename = "footer-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_decl: Option<String>,
    #[serde(rename = "date-time-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time_decl: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type OfficeScript = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeScriptAttlist {
    #[serde(rename = "@script:language")]
    pub language: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeScripts {
    #[serde(rename = "scripts")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scripts: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeSettings {
    #[serde(rename = "settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeSpreadsheetAttlist {
    #[serde(rename = "@table:structure-protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure_protected: Option<Boolean>,
    #[serde(rename = "@table:protection-key")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key: Option<String>,
    #[serde(rename = "@table:protection-key-digest-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key_digest_algorithm: Option<AnyIRI>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeSpreadsheetContentEpilogue {
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeSpreadsheetContentMain {
    #[serde(rename = "table")]
    pub table: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeSpreadsheetContentPrelude {
    #[serde(rename = "tracked-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_changes: Option<String>,
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeStyles {
    #[serde(rename = "styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeTextAttlist {
    #[serde(rename = "@text:global")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global: Option<Boolean>,
    #[serde(rename = "@text:use-soft-page-breaks")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_soft_page_breaks: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeTextContentEpilogue {
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeTextContentMain {
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    #[serde(rename = "numbered-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_paragraph: Option<String>,
    #[serde(rename = "table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "section")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-of-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_of_content: Option<String>,
    #[serde(rename = "illustration-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration_index: Option<String>,
    #[serde(rename = "table-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_index: Option<String>,
    #[serde(rename = "object-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<String>,
    #[serde(rename = "user-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index: Option<String>,
    #[serde(rename = "alphabetical-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index: Option<String>,
    #[serde(rename = "bibliography")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<String>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    #[serde(rename = "page-sequence")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_sequence: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfficeTextContentPrelude {
    #[serde(rename = "forms")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forms: Option<String>,
    #[serde(rename = "tracked-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_changes: Option<String>,
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParagraphAttrs {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:class-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_names: Option<StyleNameRefs>,
    #[serde(rename = "@text:cond-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cond_style_name: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@xhtml:about")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<URIorSafeCURIE>,
    #[serde(rename = "@xhtml:property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<CURIEs>,
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParagraphContent {
    #[serde(rename = "s")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<String>,
    #[serde(rename = "tab")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<TextTabAttr>,
    #[serde(rename = "line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<()>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "span")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    #[serde(rename = "meta")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    #[serde(rename = "bookmark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
    #[serde(rename = "bookmark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_start: Option<String>,
    #[serde(rename = "bookmark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_end: Option<String>,
    #[serde(rename = "reference-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark: Option<String>,
    #[serde(rename = "reference-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark_start: Option<String>,
    #[serde(rename = "reference-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark_end: Option<String>,
    #[serde(rename = "note")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(rename = "ruby")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
    #[serde(rename = "annotation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    #[serde(rename = "annotation-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_end: Option<OfficeAnnotationEndAttlist>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(rename = "page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "page-continuation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_continuation: Option<String>,
    #[serde(rename = "sender-firstname")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_firstname: Option<String>,
    #[serde(rename = "sender-lastname")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_lastname: Option<String>,
    #[serde(rename = "sender-initials")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_initials: Option<String>,
    #[serde(rename = "sender-title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_title: Option<String>,
    #[serde(rename = "sender-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_position: Option<String>,
    #[serde(rename = "sender-email")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_email: Option<String>,
    #[serde(rename = "sender-phone-private")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_phone_private: Option<String>,
    #[serde(rename = "sender-fax")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_fax: Option<String>,
    #[serde(rename = "sender-company")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_company: Option<String>,
    #[serde(rename = "sender-phone-work")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_phone_work: Option<String>,
    #[serde(rename = "sender-street")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_street: Option<String>,
    #[serde(rename = "sender-city")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_city: Option<String>,
    #[serde(rename = "sender-postal-code")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_postal_code: Option<String>,
    #[serde(rename = "sender-country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_country: Option<String>,
    #[serde(rename = "sender-state-or-province")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_state_or_province: Option<String>,
    #[serde(rename = "author-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(rename = "author-initials")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_initials: Option<String>,
    #[serde(rename = "chapter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
    #[serde(rename = "file-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(rename = "template-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(rename = "sheet-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(rename = "variable-set")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_set: Option<String>,
    #[serde(rename = "variable-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_get: Option<String>,
    #[serde(rename = "variable-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_input: Option<String>,
    #[serde(rename = "user-field-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_get: Option<String>,
    #[serde(rename = "user-field-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_input: Option<String>,
    #[serde(rename = "sequence")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "expression")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(rename = "text-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_input: Option<String>,
    #[serde(rename = "drop-down")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_down: Option<String>,
    #[serde(rename = "initial-creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_creator: Option<String>,
    #[serde(rename = "creation-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,
    #[serde(rename = "creation-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_time: Option<String>,
    #[serde(rename = "description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "user-defined")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined: Option<String>,
    #[serde(rename = "print-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_time: Option<String>,
    #[serde(rename = "print-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_date: Option<String>,
    #[serde(rename = "printed-by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_by: Option<String>,
    #[serde(rename = "title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "subject")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "keywords")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(rename = "editing-cycles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_cycles: Option<String>,
    #[serde(rename = "editing-duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_duration: Option<String>,
    #[serde(rename = "modification-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_time: Option<String>,
    #[serde(rename = "modification-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_date: Option<String>,
    #[serde(rename = "creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(rename = "database-display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_display: Option<String>,
    #[serde(rename = "database-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_next: Option<TextDatabaseNextAttlist>,
    #[serde(rename = "database-row-select")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_row_select: Option<TextDatabaseRowSelectAttlist>,
    #[serde(rename = "database-row-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_row_number: Option<String>,
    #[serde(rename = "database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "page-variable-set")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_variable_set: Option<String>,
    #[serde(rename = "page-variable-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_variable_get: Option<String>,
    #[serde(rename = "placeholder")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(rename = "conditional-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditional_text: Option<String>,
    #[serde(rename = "hidden-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_text: Option<String>,
    #[serde(rename = "note-ref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_ref: Option<String>,
    #[serde(rename = "sequence-ref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_ref: Option<String>,
    #[serde(rename = "script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(rename = "execute-macro")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execute_macro: Option<String>,
    #[serde(rename = "hidden-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_paragraph: Option<String>,
    #[serde(rename = "dde-connection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection: Option<String>,
    #[serde(rename = "table-formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_formula: Option<String>,
    #[serde(rename = "meta-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_field: Option<String>,
    #[serde(rename = "toc-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark_start: Option<TextTocMarkStartAttrs>,
    #[serde(rename = "toc-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark_end: Option<TextId>,
    #[serde(rename = "toc-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark: Option<String>,
    #[serde(rename = "user-index-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark_start: Option<String>,
    #[serde(rename = "user-index-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark_end: Option<TextId>,
    #[serde(rename = "user-index-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark: Option<String>,
    #[serde(rename = "alphabetical-index-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark_start: Option<String>,
    #[serde(rename = "alphabetical-index-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark_end: Option<TextId>,
    #[serde(rename = "alphabetical-index-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark: Option<String>,
    #[serde(rename = "bibliography-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography_mark: Option<String>,
    #[serde(rename = "header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<()>,
    #[serde(rename = "footer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<()>,
    #[serde(rename = "date-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time: Option<()>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParagraphContentOrHyperlink {
    #[serde(rename = "s")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<String>,
    #[serde(rename = "tab")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<TextTabAttr>,
    #[serde(rename = "line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<()>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "span")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    #[serde(rename = "meta")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    #[serde(rename = "bookmark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
    #[serde(rename = "bookmark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_start: Option<String>,
    #[serde(rename = "bookmark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_end: Option<String>,
    #[serde(rename = "reference-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark: Option<String>,
    #[serde(rename = "reference-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark_start: Option<String>,
    #[serde(rename = "reference-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_mark_end: Option<String>,
    #[serde(rename = "note")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(rename = "ruby")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
    #[serde(rename = "annotation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    #[serde(rename = "annotation-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_end: Option<OfficeAnnotationEndAttlist>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(rename = "page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "page-continuation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_continuation: Option<String>,
    #[serde(rename = "sender-firstname")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_firstname: Option<String>,
    #[serde(rename = "sender-lastname")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_lastname: Option<String>,
    #[serde(rename = "sender-initials")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_initials: Option<String>,
    #[serde(rename = "sender-title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_title: Option<String>,
    #[serde(rename = "sender-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_position: Option<String>,
    #[serde(rename = "sender-email")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_email: Option<String>,
    #[serde(rename = "sender-phone-private")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_phone_private: Option<String>,
    #[serde(rename = "sender-fax")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_fax: Option<String>,
    #[serde(rename = "sender-company")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_company: Option<String>,
    #[serde(rename = "sender-phone-work")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_phone_work: Option<String>,
    #[serde(rename = "sender-street")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_street: Option<String>,
    #[serde(rename = "sender-city")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_city: Option<String>,
    #[serde(rename = "sender-postal-code")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_postal_code: Option<String>,
    #[serde(rename = "sender-country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_country: Option<String>,
    #[serde(rename = "sender-state-or-province")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_state_or_province: Option<String>,
    #[serde(rename = "author-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(rename = "author-initials")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_initials: Option<String>,
    #[serde(rename = "chapter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
    #[serde(rename = "file-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(rename = "template-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(rename = "sheet-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(rename = "variable-set")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_set: Option<String>,
    #[serde(rename = "variable-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_get: Option<String>,
    #[serde(rename = "variable-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_input: Option<String>,
    #[serde(rename = "user-field-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_get: Option<String>,
    #[serde(rename = "user-field-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_input: Option<String>,
    #[serde(rename = "sequence")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "expression")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(rename = "text-input")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_input: Option<String>,
    #[serde(rename = "drop-down")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_down: Option<String>,
    #[serde(rename = "initial-creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_creator: Option<String>,
    #[serde(rename = "creation-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,
    #[serde(rename = "creation-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_time: Option<String>,
    #[serde(rename = "description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "user-defined")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined: Option<String>,
    #[serde(rename = "print-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_time: Option<String>,
    #[serde(rename = "print-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_date: Option<String>,
    #[serde(rename = "printed-by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_by: Option<String>,
    #[serde(rename = "title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "subject")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "keywords")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(rename = "editing-cycles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_cycles: Option<String>,
    #[serde(rename = "editing-duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_duration: Option<String>,
    #[serde(rename = "modification-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_time: Option<String>,
    #[serde(rename = "modification-date")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_date: Option<String>,
    #[serde(rename = "creator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(rename = "database-display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_display: Option<String>,
    #[serde(rename = "database-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_next: Option<TextDatabaseNextAttlist>,
    #[serde(rename = "database-row-select")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_row_select: Option<TextDatabaseRowSelectAttlist>,
    #[serde(rename = "database-row-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_row_number: Option<String>,
    #[serde(rename = "database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "page-variable-set")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_variable_set: Option<String>,
    #[serde(rename = "page-variable-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_variable_get: Option<String>,
    #[serde(rename = "placeholder")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(rename = "conditional-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditional_text: Option<String>,
    #[serde(rename = "hidden-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_text: Option<String>,
    #[serde(rename = "note-ref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_ref: Option<String>,
    #[serde(rename = "sequence-ref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_ref: Option<String>,
    #[serde(rename = "script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(rename = "execute-macro")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execute_macro: Option<String>,
    #[serde(rename = "hidden-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_paragraph: Option<String>,
    #[serde(rename = "dde-connection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection: Option<String>,
    #[serde(rename = "table-formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_formula: Option<String>,
    #[serde(rename = "meta-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_field: Option<String>,
    #[serde(rename = "toc-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark_start: Option<TextTocMarkStartAttrs>,
    #[serde(rename = "toc-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark_end: Option<TextId>,
    #[serde(rename = "toc-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toc_mark: Option<String>,
    #[serde(rename = "user-index-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark_start: Option<String>,
    #[serde(rename = "user-index-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark_end: Option<TextId>,
    #[serde(rename = "user-index-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index_mark: Option<String>,
    #[serde(rename = "alphabetical-index-mark-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark_start: Option<String>,
    #[serde(rename = "alphabetical-index-mark-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark_end: Option<TextId>,
    #[serde(rename = "alphabetical-index-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_mark: Option<String>,
    #[serde(rename = "bibliography-mark")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography_mark: Option<String>,
    #[serde(rename = "header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<()>,
    #[serde(rename = "footer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<()>,
    #[serde(rename = "date-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time: Option<()>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationAnimationElements {
    #[serde(rename = "show-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_shape: Option<String>,
    #[serde(rename = "show-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_text: Option<String>,
    #[serde(rename = "hide-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_shape: Option<String>,
    #[serde(rename = "hide-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_text: Option<String>,
    #[serde(rename = "dim")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim: Option<String>,
    #[serde(rename = "play")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type PresentationAnimationGroup = String;

pub type PresentationAnimations = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationDateTimeDeclAttlist {
    #[serde(rename = "@presentation:name")]
    pub name: String,
    #[serde(rename = "@presentation:source")]
    pub source: String,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationDecl {
    #[serde(rename = "header-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_decl: Option<String>,
    #[serde(rename = "footer-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_decl: Option<String>,
    #[serde(rename = "date-time-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time_decl: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationDecls {
    #[serde(rename = "header-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_decl: Option<String>,
    #[serde(rename = "footer-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_decl: Option<String>,
    #[serde(rename = "date-time-decl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_time_decl: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type PresentationDim = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationDimAttlist {
    #[serde(rename = "@draw:shape-id")]
    pub shape_id: IDREF,
    #[serde(rename = "@draw:color")]
    pub color: Color,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type PresentationEventListener = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationEventListenerAttlist {
    #[serde(rename = "@script:event-name")]
    pub event_name: String,
    #[serde(rename = "@presentation:action")]
    pub action: String,
    #[serde(rename = "@presentation:effect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<PresentationEffects>,
    #[serde(rename = "@presentation:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<PresentationEffectDirections>,
    #[serde(rename = "@presentation:speed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PresentationSpeeds>,
    #[serde(rename = "@presentation:start-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_scale: Option<Percent>,
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@presentation:verb")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verb: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationFooterDeclAttlist {
    #[serde(rename = "@presentation:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationHeaderDeclAttlist {
    #[serde(rename = "@presentation:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type PresentationHideShape = String;

pub type PresentationHideText = String;

pub type PresentationNotes = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationNotesAttlist {
    #[serde(rename = "@style:page-layout-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_layout_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type PresentationPlaceholder = String;

pub type PresentationPlay = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationPlayAttlist {
    #[serde(rename = "@draw:shape-id")]
    pub shape_id: IDREF,
    #[serde(rename = "@presentation:speed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PresentationSpeeds>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationSettings {
    #[serde(rename = "settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationSettingsAttlist {
    #[serde(rename = "@presentation:start-page")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_page: Option<String>,
    #[serde(rename = "@presentation:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@presentation:full-screen")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_screen: Option<Boolean>,
    #[serde(rename = "@presentation:endless")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endless: Option<Boolean>,
    #[serde(rename = "@presentation:pause")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause: Option<Duration>,
    #[serde(rename = "@presentation:show-logo")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_logo: Option<Boolean>,
    #[serde(rename = "@presentation:force-manual")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_manual: Option<Boolean>,
    #[serde(rename = "@presentation:mouse-visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mouse_visible: Option<Boolean>,
    #[serde(rename = "@presentation:mouse-as-pen")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mouse_as_pen: Option<Boolean>,
    #[serde(rename = "@presentation:start-with-navigator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_with_navigator: Option<Boolean>,
    #[serde(rename = "@presentation:animations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animations: Option<String>,
    #[serde(rename = "@presentation:transition-on-click")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_on_click: Option<String>,
    #[serde(rename = "@presentation:stay-on-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_on_top: Option<Boolean>,
    #[serde(rename = "@presentation:show-end-of-presentation-slide")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_end_of_presentation_slide: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationShapeAttlist {
    #[serde(rename = "@presentation:class")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<PresentationClasses>,
    #[serde(rename = "@presentation:placeholder")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<Boolean>,
    #[serde(rename = "@presentation:user-transformed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_transformed: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type PresentationShow = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationShowAttlist {
    #[serde(rename = "@presentation:name")]
    pub name: String,
    #[serde(rename = "@presentation:pages")]
    pub pages: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type PresentationShowShape = String;

pub type PresentationShowText = String;

pub type PresentationSound = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationSoundAttlist {
    #[serde(rename = "@presentation:play-full")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_full: Option<Boolean>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionContent {
    #[serde(rename = "p")]
    pub p: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type ScriptEventListener = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEventListenerAttlist {
    #[serde(rename = "@script:event-name")]
    pub event_name: String,
    #[serde(rename = "@script:language")]
    pub language: String,
    #[serde(rename = "@script:macro-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macro_name: Option<String>,
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selected {
    #[serde(rename = "@form:selected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShadowType {
    #[serde(rename = "$text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shape {
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShapeInstance {
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shapes3d {
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "extrude")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrude: Option<String>,
    #[serde(rename = "sphere")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sphere: Option<String>,
    #[serde(rename = "rotate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotate: Option<String>,
    #[serde(rename = "cube")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cube: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Size {
    #[serde(rename = "@form:size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleBackgroundImage {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleBackgroundImageAttlist {
    #[serde(rename = "@style:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    #[serde(rename = "@style:position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(rename = "@style:filter-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_name: Option<String>,
    #[serde(rename = "@draw:opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<ZeroToHundredPercent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleChartProperties = StyleChartPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleChartPropertiesAttlist {
    #[serde(rename = "@chart:scale-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_text: Option<Boolean>,
    #[serde(rename = "@chart:three-dimensional")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub three_dimensional: Option<Boolean>,
    #[serde(rename = "@chart:deep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep: Option<Boolean>,
    #[serde(rename = "@chart:right-angled-axes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_angled_axes: Option<Boolean>,
    #[serde(rename = "@chart:symbol-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<String>,
    #[serde(rename = "@chart:symbol-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_name: Option<String>,
    #[serde(rename = "symbol-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_image: Option<String>,
    #[serde(rename = "@chart:symbol-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_width: Option<NonNegativeLength>,
    #[serde(rename = "@chart:symbol-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_height: Option<NonNegativeLength>,
    #[serde(rename = "@chart:sort-by-x-values")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by_x_values: Option<Boolean>,
    #[serde(rename = "@chart:vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<Boolean>,
    #[serde(rename = "@chart:connect-bars")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_bars: Option<Boolean>,
    #[serde(rename = "@chart:gap-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_width: Option<Integer>,
    #[serde(rename = "@chart:overlap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<Integer>,
    #[serde(rename = "@chart:group-bars-per-axis")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_bars_per_axis: Option<Boolean>,
    #[serde(rename = "@chart:japanese-candle-stick")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub japanese_candle_stick: Option<Boolean>,
    #[serde(rename = "@chart:interpolation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<String>,
    #[serde(rename = "@chart:spline-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spline_order: Option<PositiveInteger>,
    #[serde(rename = "@chart:spline-resolution")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spline_resolution: Option<PositiveInteger>,
    #[serde(rename = "@chart:pie-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pie_offset: Option<NonNegativeInteger>,
    #[serde(rename = "@chart:angle-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_offset: Option<Angle>,
    #[serde(rename = "@chart:hole-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<Percent>,
    #[serde(rename = "@chart:lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Boolean>,
    #[serde(rename = "@chart:solid-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid_type: Option<String>,
    #[serde(rename = "@chart:stacked")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacked: Option<Boolean>,
    #[serde(rename = "@chart:percentage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage: Option<Boolean>,
    #[serde(rename = "@chart:treat-empty-cells")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treat_empty_cells: Option<String>,
    #[serde(rename = "@chart:link-data-style-to-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_data_style_to_source: Option<Boolean>,
    #[serde(rename = "@chart:logarithmic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logarithmic: Option<Boolean>,
    #[serde(rename = "@chart:maximum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<Double>,
    #[serde(rename = "@chart:minimum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<Double>,
    #[serde(rename = "@chart:origin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Double>,
    #[serde(rename = "@chart:interval-major")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_major: Option<Double>,
    #[serde(rename = "@chart:interval-minor-divisor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_minor_divisor: Option<PositiveInteger>,
    #[serde(rename = "@chart:tick-marks-major-inner")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_major_inner: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-major-outer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_major_outer: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-minor-inner")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_minor_inner: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-minor-outer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_minor_outer: Option<Boolean>,
    #[serde(rename = "@chart:reverse-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse_direction: Option<Boolean>,
    #[serde(rename = "@chart:display-label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_label: Option<Boolean>,
    #[serde(rename = "@chart:text-overlap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overlap: Option<Boolean>,
    #[serde(rename = "@text:line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<Boolean>,
    #[serde(rename = "@chart:label-arrangement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_arrangement: Option<String>,
    #[serde(rename = "@style:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@style:rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_angle: Option<Angle>,
    #[serde(rename = "@chart:data-label-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_number: Option<String>,
    #[serde(rename = "@chart:data-label-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_text: Option<Boolean>,
    #[serde(rename = "@chart:data-label-symbol")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_symbol: Option<Boolean>,
    #[serde(rename = "label-separator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_separator: Option<TextP>,
    #[serde(rename = "@chart:label-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_position: Option<LabelPositions>,
    #[serde(rename = "@chart:label-position-negative")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_position_negative: Option<LabelPositions>,
    #[serde(rename = "@chart:visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<Boolean>,
    #[serde(rename = "@chart:auto-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_position: Option<Boolean>,
    #[serde(rename = "@chart:auto-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<Boolean>,
    #[serde(rename = "@chart:mean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_value: Option<Boolean>,
    #[serde(rename = "@chart:error-category")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_category: Option<String>,
    #[serde(rename = "@chart:error-percentage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_percentage: Option<Double>,
    #[serde(rename = "@chart:error-margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_margin: Option<Double>,
    #[serde(rename = "@chart:error-lower-limit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_limit: Option<Double>,
    #[serde(rename = "@chart:error-upper-limit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_limit: Option<Double>,
    #[serde(rename = "@chart:error-upper-indicator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_indicator: Option<Boolean>,
    #[serde(rename = "@chart:error-lower-indicator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_indicator: Option<Boolean>,
    #[serde(rename = "@chart:error-lower-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_range: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:error-upper-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_range: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:series-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_source: Option<String>,
    #[serde(rename = "@chart:regression-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_type: Option<String>,
    #[serde(rename = "@chart:regression-max-degree")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_max_degree: Option<PositiveInteger>,
    #[serde(rename = "@chart:regression-force-intercept")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_force_intercept: Option<Boolean>,
    #[serde(rename = "@chart:regression-intercept-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_intercept_value: Option<Double>,
    #[serde(rename = "@chart:regression-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_name: Option<String>,
    #[serde(rename = "@chart:regression-period")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_period: Option<PositiveInteger>,
    #[serde(rename = "@chart:regression-moving-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_moving_type: Option<String>,
    #[serde(rename = "@chart:axis-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_position: Option<String>,
    #[serde(rename = "@chart:axis-label-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_label_position: Option<String>,
    #[serde(rename = "@chart:tick-mark-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_mark_position: Option<String>,
    #[serde(rename = "@chart:include-hidden-cells")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_hidden_cells: Option<Boolean>,
    #[serde(rename = "@chart:data-label-series")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_series: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleChartPropertiesContentStrict {
    #[serde(rename = "@chart:scale-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_text: Option<Boolean>,
    #[serde(rename = "@chart:three-dimensional")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub three_dimensional: Option<Boolean>,
    #[serde(rename = "@chart:deep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep: Option<Boolean>,
    #[serde(rename = "@chart:right-angled-axes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_angled_axes: Option<Boolean>,
    #[serde(rename = "@chart:symbol-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<String>,
    #[serde(rename = "@chart:symbol-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_name: Option<String>,
    #[serde(rename = "symbol-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_image: Option<String>,
    #[serde(rename = "@chart:symbol-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_width: Option<NonNegativeLength>,
    #[serde(rename = "@chart:symbol-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_height: Option<NonNegativeLength>,
    #[serde(rename = "@chart:sort-by-x-values")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by_x_values: Option<Boolean>,
    #[serde(rename = "@chart:vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<Boolean>,
    #[serde(rename = "@chart:connect-bars")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_bars: Option<Boolean>,
    #[serde(rename = "@chart:gap-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_width: Option<Integer>,
    #[serde(rename = "@chart:overlap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<Integer>,
    #[serde(rename = "@chart:group-bars-per-axis")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_bars_per_axis: Option<Boolean>,
    #[serde(rename = "@chart:japanese-candle-stick")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub japanese_candle_stick: Option<Boolean>,
    #[serde(rename = "@chart:interpolation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<String>,
    #[serde(rename = "@chart:spline-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spline_order: Option<PositiveInteger>,
    #[serde(rename = "@chart:spline-resolution")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spline_resolution: Option<PositiveInteger>,
    #[serde(rename = "@chart:pie-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pie_offset: Option<NonNegativeInteger>,
    #[serde(rename = "@chart:angle-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_offset: Option<Angle>,
    #[serde(rename = "@chart:hole-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<Percent>,
    #[serde(rename = "@chart:lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Boolean>,
    #[serde(rename = "@chart:solid-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid_type: Option<String>,
    #[serde(rename = "@chart:stacked")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacked: Option<Boolean>,
    #[serde(rename = "@chart:percentage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage: Option<Boolean>,
    #[serde(rename = "@chart:treat-empty-cells")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treat_empty_cells: Option<String>,
    #[serde(rename = "@chart:link-data-style-to-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_data_style_to_source: Option<Boolean>,
    #[serde(rename = "@chart:logarithmic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logarithmic: Option<Boolean>,
    #[serde(rename = "@chart:maximum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<Double>,
    #[serde(rename = "@chart:minimum")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<Double>,
    #[serde(rename = "@chart:origin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Double>,
    #[serde(rename = "@chart:interval-major")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_major: Option<Double>,
    #[serde(rename = "@chart:interval-minor-divisor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_minor_divisor: Option<PositiveInteger>,
    #[serde(rename = "@chart:tick-marks-major-inner")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_major_inner: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-major-outer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_major_outer: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-minor-inner")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_minor_inner: Option<Boolean>,
    #[serde(rename = "@chart:tick-marks-minor-outer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_marks_minor_outer: Option<Boolean>,
    #[serde(rename = "@chart:reverse-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse_direction: Option<Boolean>,
    #[serde(rename = "@chart:display-label")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_label: Option<Boolean>,
    #[serde(rename = "@chart:text-overlap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overlap: Option<Boolean>,
    #[serde(rename = "@text:line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<Boolean>,
    #[serde(rename = "@chart:label-arrangement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_arrangement: Option<String>,
    #[serde(rename = "@style:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@style:rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_angle: Option<Angle>,
    #[serde(rename = "@chart:data-label-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_number: Option<String>,
    #[serde(rename = "@chart:data-label-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_text: Option<Boolean>,
    #[serde(rename = "@chart:data-label-symbol")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_symbol: Option<Boolean>,
    #[serde(rename = "label-separator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_separator: Option<TextP>,
    #[serde(rename = "@chart:label-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_position: Option<LabelPositions>,
    #[serde(rename = "@chart:label-position-negative")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_position_negative: Option<LabelPositions>,
    #[serde(rename = "@chart:visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<Boolean>,
    #[serde(rename = "@chart:auto-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_position: Option<Boolean>,
    #[serde(rename = "@chart:auto-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<Boolean>,
    #[serde(rename = "@chart:mean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_value: Option<Boolean>,
    #[serde(rename = "@chart:error-category")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_category: Option<String>,
    #[serde(rename = "@chart:error-percentage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_percentage: Option<Double>,
    #[serde(rename = "@chart:error-margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_margin: Option<Double>,
    #[serde(rename = "@chart:error-lower-limit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_limit: Option<Double>,
    #[serde(rename = "@chart:error-upper-limit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_limit: Option<Double>,
    #[serde(rename = "@chart:error-upper-indicator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_indicator: Option<Boolean>,
    #[serde(rename = "@chart:error-lower-indicator")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_indicator: Option<Boolean>,
    #[serde(rename = "@chart:error-lower-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_lower_range: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:error-upper-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_upper_range: Option<CellRangeAddressList>,
    #[serde(rename = "@chart:series-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_source: Option<String>,
    #[serde(rename = "@chart:regression-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_type: Option<String>,
    #[serde(rename = "@chart:regression-max-degree")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_max_degree: Option<PositiveInteger>,
    #[serde(rename = "@chart:regression-force-intercept")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_force_intercept: Option<Boolean>,
    #[serde(rename = "@chart:regression-intercept-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_intercept_value: Option<Double>,
    #[serde(rename = "@chart:regression-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_name: Option<String>,
    #[serde(rename = "@chart:regression-period")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_period: Option<PositiveInteger>,
    #[serde(rename = "@chart:regression-moving-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_moving_type: Option<String>,
    #[serde(rename = "@chart:axis-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_position: Option<String>,
    #[serde(rename = "@chart:axis-label-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_label_position: Option<String>,
    #[serde(rename = "@chart:tick-mark-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_mark_position: Option<String>,
    #[serde(rename = "@chart:include-hidden-cells")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_hidden_cells: Option<Boolean>,
    #[serde(rename = "@chart:data-label-series")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_label_series: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleChartPropertiesElements;

pub type StyleColumn = StyleColumnAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleColumnAttlist {
    #[serde(rename = "@style:rel-width")]
    pub rel_width: RelativeLength,
    #[serde(rename = "@fo:start-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_indent: Option<Length>,
    #[serde(rename = "@fo:end-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_indent: Option<Length>,
    #[serde(rename = "@fo:space-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(rename = "@fo:space-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleColumnSep = StyleColumnSepAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleColumnSepAttlist {
    #[serde(rename = "@style:style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(rename = "@style:width")]
    pub width: Length,
    #[serde(rename = "@style:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<ZeroToHundredPercent>,
    #[serde(rename = "@style:vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(rename = "@style:color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleColumns {
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleColumnsAttlist {
    #[serde(rename = "@fo:column-count")]
    pub column_count: PositiveInteger,
    #[serde(rename = "@fo:column-gap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_gap: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleDefaultPageLayout = StylePageLayoutContent;

pub type StyleDefaultStyle = StyleStyleContent;

pub type StyleDrawingPageProperties = StyleDrawingPagePropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleDrawingPagePropertiesAttlist {
    #[serde(rename = "@presentation:transition-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<String>,
    #[serde(rename = "@presentation:transition-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_style: Option<String>,
    #[serde(rename = "@presentation:transition-speed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_speed: Option<PresentationSpeeds>,
    #[serde(rename = "@smil:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@smil:subtype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(rename = "@smil:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@smil:fadeColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_color: Option<Color>,
    #[serde(rename = "@presentation:duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Duration>,
    #[serde(rename = "@presentation:visibility")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(rename = "@draw:background-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_size: Option<String>,
    #[serde(rename = "@presentation:background-objects-visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_objects_visible: Option<Boolean>,
    #[serde(rename = "@presentation:background-visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_visible: Option<Boolean>,
    #[serde(rename = "@presentation:display-header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_header: Option<Boolean>,
    #[serde(rename = "@presentation:display-footer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_footer: Option<Boolean>,
    #[serde(rename = "@presentation:display-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_page_number: Option<Boolean>,
    #[serde(rename = "@presentation:display-date-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_date_time: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleDrawingPagePropertiesContentStrict {
    #[serde(rename = "@draw:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(rename = "@draw:fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    #[serde(rename = "@draw:secondary-fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_fill_color: Option<Color>,
    #[serde(rename = "@draw:fill-gradient-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_gradient_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:gradient-step-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_step_count: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:fill-hatch-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:fill-hatch-solid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_solid: Option<Boolean>,
    #[serde(rename = "@draw:fill-image-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_name: Option<StyleNameRef>,
    #[serde(rename = "@style:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    #[serde(rename = "@draw:fill-image-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_width: Option<String>,
    #[serde(rename = "@draw:fill-image-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_height: Option<String>,
    #[serde(rename = "@draw:fill-image-ref-point-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_x: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_y: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point: Option<String>,
    #[serde(rename = "@draw:tile-repeat-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_repeat_offset: Option<String>,
    #[serde(rename = "@draw:opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:opacity-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity_name: Option<StyleNameRef>,
    #[serde(rename = "@svg:fill-rule")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_rule: Option<String>,
    #[serde(rename = "@presentation:transition-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<String>,
    #[serde(rename = "@presentation:transition-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_style: Option<String>,
    #[serde(rename = "@presentation:transition-speed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_speed: Option<PresentationSpeeds>,
    #[serde(rename = "@smil:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@smil:subtype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(rename = "@smil:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@smil:fadeColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_color: Option<Color>,
    #[serde(rename = "@presentation:duration")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Duration>,
    #[serde(rename = "@presentation:visibility")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(rename = "@draw:background-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_size: Option<String>,
    #[serde(rename = "@presentation:background-objects-visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_objects_visible: Option<Boolean>,
    #[serde(rename = "@presentation:background-visible")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_visible: Option<Boolean>,
    #[serde(rename = "@presentation:display-header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_header: Option<Boolean>,
    #[serde(rename = "@presentation:display-footer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_footer: Option<Boolean>,
    #[serde(rename = "@presentation:display-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_page_number: Option<Boolean>,
    #[serde(rename = "@presentation:display-date-time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_date_time: Option<Boolean>,
    #[serde(rename = "sound")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleDrawingPagePropertiesElements {
    #[serde(rename = "sound")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleDropCap {
    #[serde(rename = "drop-cap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleDropCapAttlist {
    #[serde(rename = "@style:length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<String>,
    #[serde(rename = "@style:lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<PositiveInteger>,
    #[serde(rename = "@style:distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<Length>,
    #[serde(rename = "@style:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleFontFace = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleFontFaceAttlist {
    #[serde(rename = "@svg:font-family")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(rename = "@svg:font-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(rename = "@svg:font-variant")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(rename = "@svg:font-weight")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(rename = "@svg:font-stretch")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_stretch: Option<String>,
    #[serde(rename = "@svg:font-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<PositiveLength>,
    #[serde(rename = "@svg:unicode-range")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_range: Option<String>,
    #[serde(rename = "@svg:units-per-em")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_per_em: Option<Integer>,
    #[serde(rename = "@svg:panose-1")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panose_1: Option<String>,
    #[serde(rename = "@svg:stemv")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stemv: Option<Integer>,
    #[serde(rename = "@svg:stemh")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stemh: Option<Integer>,
    #[serde(rename = "@svg:slope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slope: Option<Integer>,
    #[serde(rename = "@svg:cap-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_height: Option<Integer>,
    #[serde(rename = "@svg:x-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_height: Option<Integer>,
    #[serde(rename = "@svg:accent-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_height: Option<Integer>,
    #[serde(rename = "@svg:ascent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ascent: Option<Integer>,
    #[serde(rename = "@svg:descent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descent: Option<Integer>,
    #[serde(rename = "@svg:widths")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widths: Option<String>,
    #[serde(rename = "@svg:bbox")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<String>,
    #[serde(rename = "@svg:ideographic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ideographic: Option<Integer>,
    #[serde(rename = "@svg:alphabetic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetic: Option<Integer>,
    #[serde(rename = "@svg:mathematical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mathematical: Option<Integer>,
    #[serde(rename = "@svg:hanging")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hanging: Option<Integer>,
    #[serde(rename = "@svg:v-ideographic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_ideographic: Option<Integer>,
    #[serde(rename = "@svg:v-alphabetic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_alphabetic: Option<Integer>,
    #[serde(rename = "@svg:v-mathematical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_mathematical: Option<Integer>,
    #[serde(rename = "@svg:v-hanging")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_hanging: Option<Integer>,
    #[serde(rename = "@svg:underline-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline_position: Option<Integer>,
    #[serde(rename = "@svg:underline-thickness")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline_thickness: Option<Integer>,
    #[serde(rename = "@svg:strikethrough-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough_position: Option<Integer>,
    #[serde(rename = "@svg:strikethrough-thickness")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough_thickness: Option<Integer>,
    #[serde(rename = "@svg:overline-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overline_position: Option<Integer>,
    #[serde(rename = "@svg:overline-thickness")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overline_thickness: Option<Integer>,
    #[serde(rename = "@style:name")]
    pub name: String,
    #[serde(rename = "@style:font-adornments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_adornments: Option<String>,
    #[serde(rename = "@style:font-family-generic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-pitch")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch: Option<FontPitch>,
    #[serde(rename = "@style:font-charset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset: Option<TextEncoding>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleFooter = String;

pub type StyleFooterFirst = String;

pub type StyleFooterLeft = String;

pub type StyleFooterStyle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleFootnoteSep {
    #[serde(rename = "footnote-sep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_sep: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleFootnoteSepAttlist {
    #[serde(rename = "@style:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<Percent>,
    #[serde(rename = "@style:color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(rename = "@style:line-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_style: Option<LineStyle>,
    #[serde(rename = "@style:adjustment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment: Option<String>,
    #[serde(rename = "@style:distance-before-sep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_before_sep: Option<Length>,
    #[serde(rename = "@style:distance-after-sep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_after_sep: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleGraphicFillPropertiesAttlist {
    #[serde(rename = "@draw:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(rename = "@draw:fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    #[serde(rename = "@draw:secondary-fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_fill_color: Option<Color>,
    #[serde(rename = "@draw:fill-gradient-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_gradient_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:gradient-step-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_step_count: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:fill-hatch-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:fill-hatch-solid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_solid: Option<Boolean>,
    #[serde(rename = "@draw:fill-image-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_name: Option<StyleNameRef>,
    #[serde(rename = "@style:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    #[serde(rename = "@draw:fill-image-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_width: Option<String>,
    #[serde(rename = "@draw:fill-image-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_height: Option<String>,
    #[serde(rename = "@draw:fill-image-ref-point-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_x: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_y: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point: Option<String>,
    #[serde(rename = "@draw:tile-repeat-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_repeat_offset: Option<String>,
    #[serde(rename = "@draw:opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:opacity-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity_name: Option<StyleNameRef>,
    #[serde(rename = "@svg:fill-rule")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_rule: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleGraphicProperties = StyleGraphicPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleGraphicPropertiesAttlist {
    #[serde(rename = "@draw:stroke")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(rename = "@draw:stroke-dash")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash: Option<StyleNameRef>,
    #[serde(rename = "@draw:stroke-dash-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash_names: Option<StyleNameRefs>,
    #[serde(rename = "@svg:stroke-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<Length>,
    #[serde(rename = "@svg:stroke-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<Color>,
    #[serde(rename = "@draw:marker-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start: Option<StyleNameRef>,
    #[serde(rename = "@draw:marker-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end: Option<StyleNameRef>,
    #[serde(rename = "@draw:marker-start-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start_width: Option<Length>,
    #[serde(rename = "@draw:marker-end-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end_width: Option<Length>,
    #[serde(rename = "@draw:marker-start-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start_center: Option<Boolean>,
    #[serde(rename = "@draw:marker-end-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end_center: Option<Boolean>,
    #[serde(rename = "@svg:stroke-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_opacity: Option<String>,
    #[serde(rename = "@draw:stroke-linejoin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_linejoin: Option<String>,
    #[serde(rename = "@svg:stroke-linecap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_linecap: Option<String>,
    #[serde(rename = "@draw:symbol-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_color: Option<Color>,
    #[serde(rename = "@text:animation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    #[serde(rename = "@text:animation-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_direction: Option<String>,
    #[serde(rename = "@text:animation-start-inside")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_start_inside: Option<Boolean>,
    #[serde(rename = "@text:animation-stop-inside")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_stop_inside: Option<Boolean>,
    #[serde(rename = "@text:animation-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_repeat: Option<NonNegativeInteger>,
    #[serde(rename = "@text:animation-delay")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_delay: Option<Duration>,
    #[serde(rename = "@text:animation-steps")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_steps: Option<Length>,
    #[serde(rename = "@draw:auto-grow-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_grow_width: Option<Boolean>,
    #[serde(rename = "@draw:auto-grow-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_grow_height: Option<Boolean>,
    #[serde(rename = "@draw:fit-to-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_to_size: Option<Boolean>,
    #[serde(rename = "@draw:fit-to-contour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_to_contour: Option<Boolean>,
    #[serde(rename = "@draw:textarea-vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea_vertical_align: Option<String>,
    #[serde(rename = "@draw:textarea-horizontal-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea_horizontal_align: Option<String>,
    #[serde(rename = "@fo:wrap-option")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_option: Option<String>,
    #[serde(rename = "@style:shrink-to-fit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<Boolean>,
    #[serde(rename = "@draw:color-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_mode: Option<String>,
    #[serde(rename = "@draw:color-inversion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_inversion: Option<Boolean>,
    #[serde(rename = "@draw:luminance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luminance: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:contrast")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<Percent>,
    #[serde(rename = "@draw:gamma")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamma: Option<Percent>,
    #[serde(rename = "@draw:red")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:green")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:blue")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:image-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<String>,
    #[serde(rename = "@draw:shadow-offset-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_offset_x: Option<Length>,
    #[serde(rename = "@draw:shadow-offset-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_offset_y: Option<Length>,
    #[serde(rename = "@draw:shadow-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<Color>,
    #[serde(rename = "@draw:shadow-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:start-line-spacing-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line_spacing_horizontal: Option<Distance>,
    #[serde(rename = "@draw:start-line-spacing-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line_spacing_vertical: Option<Distance>,
    #[serde(rename = "@draw:end-line-spacing-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line_spacing_horizontal: Option<Distance>,
    #[serde(rename = "@draw:end-line-spacing-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line_spacing_vertical: Option<Distance>,
    #[serde(rename = "@draw:line-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_distance: Option<Distance>,
    #[serde(rename = "@draw:guide-overhang")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_overhang: Option<Length>,
    #[serde(rename = "@draw:guide-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_distance: Option<Distance>,
    #[serde(rename = "@draw:start-guide")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_guide: Option<Length>,
    #[serde(rename = "@draw:end-guide")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_guide: Option<Length>,
    #[serde(rename = "@draw:placing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placing: Option<String>,
    #[serde(rename = "@draw:parallel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel: Option<Boolean>,
    #[serde(rename = "@draw:measure-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure_align: Option<String>,
    #[serde(rename = "@draw:measure-vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure_vertical_align: Option<String>,
    #[serde(rename = "@draw:unit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(rename = "@draw:show-unit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_unit: Option<Boolean>,
    #[serde(rename = "@draw:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:caption-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_type: Option<String>,
    #[serde(rename = "@draw:caption-angle-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_angle_type: Option<String>,
    #[serde(rename = "@draw:caption-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_angle: Option<Angle>,
    #[serde(rename = "@draw:caption-gap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_gap: Option<Distance>,
    #[serde(rename = "@draw:caption-escape-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_escape_direction: Option<String>,
    #[serde(rename = "@draw:caption-escape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_escape: Option<String>,
    #[serde(rename = "@draw:caption-line-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_line_length: Option<Length>,
    #[serde(rename = "@draw:caption-fit-line-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_fit_line_length: Option<Boolean>,
    #[serde(rename = "@dr3d:horizontal-segments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_segments: Option<NonNegativeInteger>,
    #[serde(rename = "@dr3d:vertical-segments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_segments: Option<NonNegativeInteger>,
    #[serde(rename = "@dr3d:edge-rounding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding: Option<Percent>,
    #[serde(rename = "@dr3d:edge-rounding-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding_mode: Option<String>,
    #[serde(rename = "@dr3d:back-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_scale: Option<Percent>,
    #[serde(rename = "@dr3d:depth")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Length>,
    #[serde(rename = "@dr3d:backface-culling")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backface_culling: Option<String>,
    #[serde(rename = "@dr3d:end-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_angle: Option<Angle>,
    #[serde(rename = "@dr3d:close-front")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_front: Option<Boolean>,
    #[serde(rename = "@dr3d:close-back")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_back: Option<Boolean>,
    #[serde(rename = "@dr3d:lighting-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting_mode: Option<String>,
    #[serde(rename = "@dr3d:normals-kind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normals_kind: Option<String>,
    #[serde(rename = "@dr3d:normals-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normals_direction: Option<String>,
    #[serde(rename = "@dr3d:texture-generation-mode-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_generation_mode_x: Option<String>,
    #[serde(rename = "@dr3d:texture-generation-mode-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_generation_mode_y: Option<String>,
    #[serde(rename = "@dr3d:texture-kind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_kind: Option<String>,
    #[serde(rename = "@dr3d:texture-filter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_filter: Option<String>,
    #[serde(rename = "@dr3d:texture-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_mode: Option<String>,
    #[serde(rename = "@dr3d:ambient-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<Color>,
    #[serde(rename = "@dr3d:emissive-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_color: Option<Color>,
    #[serde(rename = "@dr3d:specular-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specular_color: Option<Color>,
    #[serde(rename = "@dr3d:diffuse-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffuse_color: Option<Color>,
    #[serde(rename = "@dr3d:shininess")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shininess: Option<Percent>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<String>,
    #[serde(rename = "@style:rel-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_height: Option<String>,
    #[serde(rename = "@fo:min-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,
    #[serde(rename = "@fo:min-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<String>,
    #[serde(rename = "@fo:max-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<String>,
    #[serde(rename = "@fo:max-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@style:print-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_content: Option<Boolean>,
    #[serde(rename = "@style:protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<String>,
    #[serde(rename = "@style:horizontal-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_pos: Option<String>,
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@style:horizontal-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_rel: Option<String>,
    #[serde(rename = "@style:vertical-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_pos: Option<String>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@style:vertical-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_rel: Option<String>,
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:background-transparency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<ZeroToHundredPercent>,
    #[serde(rename = "@style:editable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable: Option<Boolean>,
    #[serde(rename = "@style:wrap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
    #[serde(rename = "@style:wrap-dynamic-threshold")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_dynamic_threshold: Option<NonNegativeLength>,
    #[serde(rename = "@style:number-wrapped-paragraphs")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_wrapped_paragraphs: Option<String>,
    #[serde(rename = "@style:wrap-contour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_contour: Option<Boolean>,
    #[serde(rename = "@style:wrap-contour-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_contour_mode: Option<String>,
    #[serde(rename = "@style:run-through")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_through: Option<String>,
    #[serde(rename = "@style:flow-with-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_with_text: Option<Boolean>,
    #[serde(rename = "@style:overflow-behavior")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overflow_behavior: Option<String>,
    #[serde(rename = "@style:mirror")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
    #[serde(rename = "@fo:clip")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    #[serde(rename = "@draw:wrap-influence-on-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_influence_on_position: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@draw:frame-display-scrollbar")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_display_scrollbar: Option<Boolean>,
    #[serde(rename = "@draw:frame-display-border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_display_border: Option<Boolean>,
    #[serde(rename = "@draw:frame-margin-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_margin_horizontal: Option<NonNegativePixelLength>,
    #[serde(rename = "@draw:frame-margin-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_margin_vertical: Option<NonNegativePixelLength>,
    #[serde(rename = "@draw:visible-area-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_left: Option<NonNegativeLength>,
    #[serde(rename = "@draw:visible-area-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_top: Option<NonNegativeLength>,
    #[serde(rename = "@draw:visible-area-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_width: Option<PositiveLength>,
    #[serde(rename = "@draw:visible-area-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_height: Option<PositiveLength>,
    #[serde(rename = "@draw:draw-aspect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draw_aspect: Option<String>,
    #[serde(rename = "@draw:ole-draw-aspect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ole_draw_aspect: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleGraphicPropertiesContentStrict {
    #[serde(rename = "@draw:stroke")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(rename = "@draw:stroke-dash")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash: Option<StyleNameRef>,
    #[serde(rename = "@draw:stroke-dash-names")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash_names: Option<StyleNameRefs>,
    #[serde(rename = "@svg:stroke-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<Length>,
    #[serde(rename = "@svg:stroke-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<Color>,
    #[serde(rename = "@draw:marker-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start: Option<StyleNameRef>,
    #[serde(rename = "@draw:marker-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end: Option<StyleNameRef>,
    #[serde(rename = "@draw:marker-start-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start_width: Option<Length>,
    #[serde(rename = "@draw:marker-end-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end_width: Option<Length>,
    #[serde(rename = "@draw:marker-start-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start_center: Option<Boolean>,
    #[serde(rename = "@draw:marker-end-center")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end_center: Option<Boolean>,
    #[serde(rename = "@svg:stroke-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_opacity: Option<String>,
    #[serde(rename = "@draw:stroke-linejoin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_linejoin: Option<String>,
    #[serde(rename = "@svg:stroke-linecap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_linecap: Option<String>,
    #[serde(rename = "@draw:symbol-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_color: Option<Color>,
    #[serde(rename = "@text:animation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    #[serde(rename = "@text:animation-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_direction: Option<String>,
    #[serde(rename = "@text:animation-start-inside")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_start_inside: Option<Boolean>,
    #[serde(rename = "@text:animation-stop-inside")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_stop_inside: Option<Boolean>,
    #[serde(rename = "@text:animation-repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_repeat: Option<NonNegativeInteger>,
    #[serde(rename = "@text:animation-delay")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_delay: Option<Duration>,
    #[serde(rename = "@text:animation-steps")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_steps: Option<Length>,
    #[serde(rename = "@draw:auto-grow-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_grow_width: Option<Boolean>,
    #[serde(rename = "@draw:auto-grow-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_grow_height: Option<Boolean>,
    #[serde(rename = "@draw:fit-to-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_to_size: Option<Boolean>,
    #[serde(rename = "@draw:fit-to-contour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_to_contour: Option<Boolean>,
    #[serde(rename = "@draw:textarea-vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea_vertical_align: Option<String>,
    #[serde(rename = "@draw:textarea-horizontal-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textarea_horizontal_align: Option<String>,
    #[serde(rename = "@fo:wrap-option")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_option: Option<String>,
    #[serde(rename = "@style:shrink-to-fit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<Boolean>,
    #[serde(rename = "@draw:color-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_mode: Option<String>,
    #[serde(rename = "@draw:color-inversion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_inversion: Option<Boolean>,
    #[serde(rename = "@draw:luminance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luminance: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:contrast")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<Percent>,
    #[serde(rename = "@draw:gamma")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamma: Option<Percent>,
    #[serde(rename = "@draw:red")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:green")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:blue")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<SignedZeroToHundredPercent>,
    #[serde(rename = "@draw:image-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<String>,
    #[serde(rename = "@draw:shadow-offset-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_offset_x: Option<Length>,
    #[serde(rename = "@draw:shadow-offset-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_offset_y: Option<Length>,
    #[serde(rename = "@draw:shadow-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<Color>,
    #[serde(rename = "@draw:shadow-opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:start-line-spacing-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line_spacing_horizontal: Option<Distance>,
    #[serde(rename = "@draw:start-line-spacing-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line_spacing_vertical: Option<Distance>,
    #[serde(rename = "@draw:end-line-spacing-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line_spacing_horizontal: Option<Distance>,
    #[serde(rename = "@draw:end-line-spacing-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line_spacing_vertical: Option<Distance>,
    #[serde(rename = "@draw:line-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_distance: Option<Distance>,
    #[serde(rename = "@draw:guide-overhang")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_overhang: Option<Length>,
    #[serde(rename = "@draw:guide-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_distance: Option<Distance>,
    #[serde(rename = "@draw:start-guide")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_guide: Option<Length>,
    #[serde(rename = "@draw:end-guide")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_guide: Option<Length>,
    #[serde(rename = "@draw:placing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placing: Option<String>,
    #[serde(rename = "@draw:parallel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel: Option<Boolean>,
    #[serde(rename = "@draw:measure-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure_align: Option<String>,
    #[serde(rename = "@draw:measure-vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure_vertical_align: Option<String>,
    #[serde(rename = "@draw:unit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(rename = "@draw:show-unit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_unit: Option<Boolean>,
    #[serde(rename = "@draw:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:caption-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_type: Option<String>,
    #[serde(rename = "@draw:caption-angle-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_angle_type: Option<String>,
    #[serde(rename = "@draw:caption-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_angle: Option<Angle>,
    #[serde(rename = "@draw:caption-gap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_gap: Option<Distance>,
    #[serde(rename = "@draw:caption-escape-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_escape_direction: Option<String>,
    #[serde(rename = "@draw:caption-escape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_escape: Option<String>,
    #[serde(rename = "@draw:caption-line-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_line_length: Option<Length>,
    #[serde(rename = "@draw:caption-fit-line-length")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_fit_line_length: Option<Boolean>,
    #[serde(rename = "@dr3d:horizontal-segments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_segments: Option<NonNegativeInteger>,
    #[serde(rename = "@dr3d:vertical-segments")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_segments: Option<NonNegativeInteger>,
    #[serde(rename = "@dr3d:edge-rounding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding: Option<Percent>,
    #[serde(rename = "@dr3d:edge-rounding-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding_mode: Option<String>,
    #[serde(rename = "@dr3d:back-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_scale: Option<Percent>,
    #[serde(rename = "@dr3d:depth")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Length>,
    #[serde(rename = "@dr3d:backface-culling")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backface_culling: Option<String>,
    #[serde(rename = "@dr3d:end-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_angle: Option<Angle>,
    #[serde(rename = "@dr3d:close-front")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_front: Option<Boolean>,
    #[serde(rename = "@dr3d:close-back")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_back: Option<Boolean>,
    #[serde(rename = "@dr3d:lighting-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting_mode: Option<String>,
    #[serde(rename = "@dr3d:normals-kind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normals_kind: Option<String>,
    #[serde(rename = "@dr3d:normals-direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normals_direction: Option<String>,
    #[serde(rename = "@dr3d:texture-generation-mode-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_generation_mode_x: Option<String>,
    #[serde(rename = "@dr3d:texture-generation-mode-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_generation_mode_y: Option<String>,
    #[serde(rename = "@dr3d:texture-kind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_kind: Option<String>,
    #[serde(rename = "@dr3d:texture-filter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_filter: Option<String>,
    #[serde(rename = "@dr3d:texture-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_mode: Option<String>,
    #[serde(rename = "@dr3d:ambient-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<Color>,
    #[serde(rename = "@dr3d:emissive-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_color: Option<Color>,
    #[serde(rename = "@dr3d:specular-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specular_color: Option<Color>,
    #[serde(rename = "@dr3d:diffuse-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffuse_color: Option<Color>,
    #[serde(rename = "@dr3d:shininess")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shininess: Option<Percent>,
    #[serde(rename = "@svg:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<String>,
    #[serde(rename = "@style:rel-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_height: Option<String>,
    #[serde(rename = "@fo:min-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,
    #[serde(rename = "@fo:min-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<String>,
    #[serde(rename = "@fo:max-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<String>,
    #[serde(rename = "@fo:max-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@style:print-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_content: Option<Boolean>,
    #[serde(rename = "@style:protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<String>,
    #[serde(rename = "@style:horizontal-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_pos: Option<String>,
    #[serde(rename = "@svg:x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Coordinate>,
    #[serde(rename = "@style:horizontal-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_rel: Option<String>,
    #[serde(rename = "@style:vertical-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_pos: Option<String>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@style:vertical-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_rel: Option<String>,
    #[serde(rename = "@text:anchor-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    #[serde(rename = "@text:anchor-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_page_number: Option<PositiveInteger>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:background-transparency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<ZeroToHundredPercent>,
    #[serde(rename = "@style:editable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable: Option<Boolean>,
    #[serde(rename = "@style:wrap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
    #[serde(rename = "@style:wrap-dynamic-threshold")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_dynamic_threshold: Option<NonNegativeLength>,
    #[serde(rename = "@style:number-wrapped-paragraphs")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_wrapped_paragraphs: Option<String>,
    #[serde(rename = "@style:wrap-contour")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_contour: Option<Boolean>,
    #[serde(rename = "@style:wrap-contour-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_contour_mode: Option<String>,
    #[serde(rename = "@style:run-through")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_through: Option<String>,
    #[serde(rename = "@style:flow-with-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_with_text: Option<Boolean>,
    #[serde(rename = "@style:overflow-behavior")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overflow_behavior: Option<String>,
    #[serde(rename = "@style:mirror")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
    #[serde(rename = "@fo:clip")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    #[serde(rename = "@draw:wrap-influence-on-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_influence_on_position: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@draw:frame-display-scrollbar")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_display_scrollbar: Option<Boolean>,
    #[serde(rename = "@draw:frame-display-border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_display_border: Option<Boolean>,
    #[serde(rename = "@draw:frame-margin-horizontal")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_margin_horizontal: Option<NonNegativePixelLength>,
    #[serde(rename = "@draw:frame-margin-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_margin_vertical: Option<NonNegativePixelLength>,
    #[serde(rename = "@draw:visible-area-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_left: Option<NonNegativeLength>,
    #[serde(rename = "@draw:visible-area-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_top: Option<NonNegativeLength>,
    #[serde(rename = "@draw:visible-area-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_width: Option<PositiveLength>,
    #[serde(rename = "@draw:visible-area-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_area_height: Option<PositiveLength>,
    #[serde(rename = "@draw:draw-aspect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draw_aspect: Option<String>,
    #[serde(rename = "@draw:ole-draw-aspect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ole_draw_aspect: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:fill")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(rename = "@draw:fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    #[serde(rename = "@draw:secondary-fill-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_fill_color: Option<Color>,
    #[serde(rename = "@draw:fill-gradient-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_gradient_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:gradient-step-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_step_count: Option<NonNegativeInteger>,
    #[serde(rename = "@draw:fill-hatch-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_name: Option<StyleNameRef>,
    #[serde(rename = "@draw:fill-hatch-solid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_hatch_solid: Option<Boolean>,
    #[serde(rename = "@draw:fill-image-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_name: Option<StyleNameRef>,
    #[serde(rename = "@style:repeat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    #[serde(rename = "@draw:fill-image-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_width: Option<String>,
    #[serde(rename = "@draw:fill-image-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_height: Option<String>,
    #[serde(rename = "@draw:fill-image-ref-point-x")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_x: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point-y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point_y: Option<Percent>,
    #[serde(rename = "@draw:fill-image-ref-point")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_image_ref_point: Option<String>,
    #[serde(rename = "@draw:tile-repeat-offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_repeat_offset: Option<String>,
    #[serde(rename = "@draw:opacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<ZeroToHundredPercent>,
    #[serde(rename = "@draw:opacity-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity_name: Option<StyleNameRef>,
    #[serde(rename = "@svg:fill-rule")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_rule: Option<String>,
    #[serde(rename = "list-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_style: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleGraphicPropertiesElements {
    #[serde(rename = "list-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_style: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleHandoutMaster = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleHandoutMasterAttlist {
    #[serde(rename = "@presentation:presentation-page-layout-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation_page_layout_name: Option<StyleNameRef>,
    #[serde(rename = "@style:page-layout-name")]
    pub page_layout_name: StyleNameRef,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleHeader = String;

pub type StyleHeaderFirst = String;

pub type StyleHeaderFooterProperties = StyleHeaderFooterPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleHeaderFooterPropertiesAttlist {
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@fo:min-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<Length>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@style:dynamic-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_spacing: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleHeaderFooterPropertiesContentStrict {
    #[serde(rename = "@svg:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    #[serde(rename = "@fo:min-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<Length>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@style:dynamic-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_spacing: Option<Boolean>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleHeaderFooterPropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleHeaderLeft = String;

pub type StyleHeaderStyle = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleListLevelLabelAlignment {
    #[serde(rename = "list-level-label-alignment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_label_alignment: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleListLevelLabelAlignmentAttlist {
    #[serde(rename = "@text:label-followed-by")]
    pub label_followed_by: String,
    #[serde(rename = "@text:list-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_tab_stop_position: Option<Length>,
    #[serde(rename = "@fo:text-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_indent: Option<Length>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleListLevelProperties = StyleListLevelPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleListLevelPropertiesAttlist {
    #[serde(rename = "@fo:text-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(rename = "@text:space-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(rename = "@text:min-label-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_label_width: Option<NonNegativeLength>,
    #[serde(rename = "@text:min-label-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_label_distance: Option<NonNegativeLength>,
    #[serde(rename = "@style:font-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(rename = "@fo:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PositiveLength>,
    #[serde(rename = "@fo:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<PositiveLength>,
    #[serde(rename = "@style:vertical-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_rel: Option<String>,
    #[serde(rename = "@style:vertical-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_pos: Option<String>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@text:list-level-position-and-space-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_position_and_space_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleListLevelPropertiesContentStrict {
    #[serde(rename = "@fo:text-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(rename = "@text:space-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(rename = "@text:min-label-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_label_width: Option<NonNegativeLength>,
    #[serde(rename = "@text:min-label-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_label_distance: Option<NonNegativeLength>,
    #[serde(rename = "@style:font-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(rename = "@fo:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PositiveLength>,
    #[serde(rename = "@fo:height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<PositiveLength>,
    #[serde(rename = "@style:vertical-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_rel: Option<String>,
    #[serde(rename = "@style:vertical-pos")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_pos: Option<String>,
    #[serde(rename = "@svg:y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Coordinate>,
    #[serde(rename = "@text:list-level-position-and-space-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_position_and_space_mode: Option<String>,
    #[serde(rename = "list-level-label-alignment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_label_alignment: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleListLevelPropertiesElements {
    #[serde(rename = "list-level-label-alignment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_label_alignment: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleMap = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleMapAttlist {
    #[serde(rename = "@style:condition")]
    pub condition: String,
    #[serde(rename = "@style:apply-style-name")]
    pub apply_style_name: StyleNameRef,
    #[serde(rename = "@style:base-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_cell_address: Option<CellAddress>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StyleMasterPage = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleMasterPageAttlist {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    #[serde(rename = "@style:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@style:page-layout-name")]
    pub page_layout_name: StyleNameRef,
    #[serde(rename = "@draw:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:next-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleNumLetterSyncAttlist {
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type StylePageLayout = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePageLayoutAttlist {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    #[serde(rename = "@style:page-usage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_usage: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StylePageLayoutContent {
    #[serde(rename = "page-layout-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_layout_properties: Option<StylePageLayoutPropertiesContentStrict>,
    #[serde(rename = "header-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_style: Option<String>,
    #[serde(rename = "footer-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_style: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StylePageLayoutProperties = StylePageLayoutPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StylePageLayoutPropertiesAttlist {
    #[serde(rename = "@fo:page-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_width: Option<Length>,
    #[serde(rename = "@fo:page-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_height: Option<Length>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@style:paper-tray-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper_tray_name: Option<String>,
    #[serde(rename = "@style:print-orientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_orientation: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:register-truth-ref-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register_truth_ref_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:print")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<String>,
    #[serde(rename = "@style:print-page-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_page_order: Option<String>,
    #[serde(rename = "@style:first-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_page_number: Option<String>,
    #[serde(rename = "@style:scale-to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to: Option<Percent>,
    #[serde(rename = "@style:scale-to-pages")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to_pages: Option<PositiveInteger>,
    #[serde(rename = "@style:scale-to-X")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to__x: Option<PositiveInteger>,
    #[serde(rename = "@style:scale-to-Y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to__y: Option<PositiveInteger>,
    #[serde(rename = "@style:table-centering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_centering: Option<String>,
    #[serde(rename = "@style:footnote-max-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_max_height: Option<Length>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:layout-grid-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_mode: Option<String>,
    #[serde(rename = "@style:layout-grid-standard-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_standard_mode: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-base-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_base_height: Option<Length>,
    #[serde(rename = "@style:layout-grid-ruby-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_ruby_height: Option<Length>,
    #[serde(rename = "@style:layout-grid-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_lines: Option<PositiveInteger>,
    #[serde(rename = "@style:layout-grid-base-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_base_width: Option<Length>,
    #[serde(rename = "@style:layout-grid-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_color: Option<Color>,
    #[serde(rename = "@style:layout-grid-ruby-below")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_ruby_below: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-print")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_print: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_display: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-snap-to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_snap_to: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StylePageLayoutPropertiesContentStrict {
    #[serde(rename = "@fo:page-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_width: Option<Length>,
    #[serde(rename = "@fo:page-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_height: Option<Length>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@style:paper-tray-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper_tray_name: Option<String>,
    #[serde(rename = "@style:print-orientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_orientation: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:register-truth-ref-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register_truth_ref_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:print")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<String>,
    #[serde(rename = "@style:print-page-order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_page_order: Option<String>,
    #[serde(rename = "@style:first-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_page_number: Option<String>,
    #[serde(rename = "@style:scale-to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to: Option<Percent>,
    #[serde(rename = "@style:scale-to-pages")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to_pages: Option<PositiveInteger>,
    #[serde(rename = "@style:scale-to-X")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to__x: Option<PositiveInteger>,
    #[serde(rename = "@style:scale-to-Y")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_to__y: Option<PositiveInteger>,
    #[serde(rename = "@style:table-centering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_centering: Option<String>,
    #[serde(rename = "@style:footnote-max-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_max_height: Option<Length>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:layout-grid-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_mode: Option<String>,
    #[serde(rename = "@style:layout-grid-standard-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_standard_mode: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-base-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_base_height: Option<Length>,
    #[serde(rename = "@style:layout-grid-ruby-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_ruby_height: Option<Length>,
    #[serde(rename = "@style:layout-grid-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_lines: Option<PositiveInteger>,
    #[serde(rename = "@style:layout-grid-base-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_base_width: Option<Length>,
    #[serde(rename = "@style:layout-grid-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_color: Option<Color>,
    #[serde(rename = "@style:layout-grid-ruby-below")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_ruby_below: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-print")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_print: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_display: Option<Boolean>,
    #[serde(rename = "@style:layout-grid-snap-to")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid_snap_to: Option<Boolean>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    #[serde(rename = "footnote-sep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_sep: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StylePageLayoutPropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    #[serde(rename = "footnote-sep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_sep: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleParagraphProperties = StyleParagraphPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleParagraphPropertiesAttlist {
    #[serde(rename = "@style:contextual-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contextual_spacing: Option<Boolean>,
    #[serde(rename = "@fo:line-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<String>,
    #[serde(rename = "@style:line-height-at-least")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height_at_least: Option<NonNegativeLength>,
    #[serde(rename = "@style:line-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<Length>,
    #[serde(rename = "@style:font-independent-line-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_independent_line_spacing: Option<Boolean>,
    #[serde(rename = "@fo:text-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(rename = "@fo:text-align-last")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align_last: Option<String>,
    #[serde(rename = "@style:justify-single-word")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_single_word: Option<Boolean>,
    #[serde(rename = "@fo:keep-together")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_together: Option<String>,
    #[serde(rename = "@fo:widows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widows: Option<NonNegativeInteger>,
    #[serde(rename = "@fo:orphans")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orphans: Option<NonNegativeInteger>,
    #[serde(rename = "@style:tab-stop-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop_distance: Option<NonNegativeLength>,
    #[serde(rename = "@fo:hyphenation-keep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_keep: Option<String>,
    #[serde(rename = "@fo:hyphenation-ladder-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_ladder_count: Option<String>,
    #[serde(rename = "@style:register-true")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register_true: Option<Boolean>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:text-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_indent: Option<String>,
    #[serde(rename = "@style:auto-text-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_text_indent: Option<Boolean>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@style:join-border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_border: Option<Boolean>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:keep-with-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<String>,
    #[serde(rename = "@text:number-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_lines: Option<Boolean>,
    #[serde(rename = "@text:line-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<NonNegativeInteger>,
    #[serde(rename = "@style:text-autospace")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_autospace: Option<String>,
    #[serde(rename = "@style:punctuation-wrap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub punctuation_wrap: Option<String>,
    #[serde(rename = "@style:line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<String>,
    #[serde(rename = "@style:vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:writing-mode-automatic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode_automatic: Option<Boolean>,
    #[serde(rename = "@style:snap-to-layout-grid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snap_to_layout_grid: Option<Boolean>,
    #[serde(rename = "@style:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "@style:background-transparency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<ZeroToHundredPercent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleParagraphPropertiesContentStrict {
    #[serde(rename = "@style:contextual-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contextual_spacing: Option<Boolean>,
    #[serde(rename = "@fo:line-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<String>,
    #[serde(rename = "@style:line-height-at-least")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height_at_least: Option<NonNegativeLength>,
    #[serde(rename = "@style:line-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<Length>,
    #[serde(rename = "@style:font-independent-line-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_independent_line_spacing: Option<Boolean>,
    #[serde(rename = "@fo:text-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(rename = "@fo:text-align-last")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align_last: Option<String>,
    #[serde(rename = "@style:justify-single-word")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_single_word: Option<Boolean>,
    #[serde(rename = "@fo:keep-together")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_together: Option<String>,
    #[serde(rename = "@fo:widows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widows: Option<NonNegativeInteger>,
    #[serde(rename = "@fo:orphans")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orphans: Option<NonNegativeInteger>,
    #[serde(rename = "@style:tab-stop-distance")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stop_distance: Option<NonNegativeLength>,
    #[serde(rename = "@fo:hyphenation-keep")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_keep: Option<String>,
    #[serde(rename = "@fo:hyphenation-ladder-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_ladder_count: Option<String>,
    #[serde(rename = "@style:register-true")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register_true: Option<Boolean>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:text-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_indent: Option<String>,
    #[serde(rename = "@style:auto-text-indent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_text_indent: Option<Boolean>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@style:join-border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_border: Option<Boolean>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:keep-with-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<String>,
    #[serde(rename = "@text:number-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_lines: Option<Boolean>,
    #[serde(rename = "@text:line-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<NonNegativeInteger>,
    #[serde(rename = "@style:text-autospace")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_autospace: Option<String>,
    #[serde(rename = "@style:punctuation-wrap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub punctuation_wrap: Option<String>,
    #[serde(rename = "@style:line-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_break: Option<String>,
    #[serde(rename = "@style:vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:writing-mode-automatic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode_automatic: Option<Boolean>,
    #[serde(rename = "@style:snap-to-layout-grid")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snap_to_layout_grid: Option<Boolean>,
    #[serde(rename = "@style:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "@style:background-transparency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<ZeroToHundredPercent>,
    #[serde(rename = "tab-stops")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stops: Option<String>,
    #[serde(rename = "drop-cap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleParagraphPropertiesElements {
    #[serde(rename = "tab-stops")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stops: Option<String>,
    #[serde(rename = "drop-cap")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StylePresentationPageLayout = String;

pub type StyleRegionCenter = RegionContent;

pub type StyleRegionLeft = RegionContent;

pub type StyleRegionRight = RegionContent;

pub type StyleRubyProperties = StyleRubyPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleRubyPropertiesAttlist {
    #[serde(rename = "@style:ruby-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby_position: Option<String>,
    #[serde(rename = "@style:ruby-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby_align: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleRubyPropertiesContentStrict {
    #[serde(rename = "@style:ruby-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby_position: Option<String>,
    #[serde(rename = "@style:ruby-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby_align: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleRubyPropertiesElements;

pub type StyleSectionProperties = StyleSectionPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleSectionPropertiesAttlist {
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@style:protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<Boolean>,
    #[serde(rename = "@style:editable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable: Option<Boolean>,
    #[serde(rename = "@text:dont-balance-text-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dont_balance_text_columns: Option<Boolean>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSectionPropertiesContentStrict {
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@style:protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<Boolean>,
    #[serde(rename = "@style:editable")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable: Option<Boolean>,
    #[serde(rename = "@text:dont-balance-text-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dont_balance_text_columns: Option<Boolean>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    #[serde(rename = "notes-configuration")]
    pub notes_configuration: TextNotesConfigurationContent,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSectionPropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(rename = "columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<String>,
    #[serde(rename = "notes-configuration")]
    pub notes_configuration: TextNotesConfigurationContent,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleStyle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleStyleAttlist {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    #[serde(rename = "@style:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@style:parent-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:next-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:list-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level: Option<String>,
    #[serde(rename = "@style:list-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_style_name: Option<String>,
    #[serde(rename = "@style:master-page-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_page_name: Option<StyleNameRef>,
    #[serde(rename = "@style:auto-update")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<Boolean>,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:percentage-data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage_data_style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:class")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(rename = "@style:default-outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_outline_level: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleStyleContent {
    #[serde(rename = "@style:family")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(rename = "text-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_properties: Option<StyleTextPropertiesContentStrict>,
    #[serde(rename = "paragraph-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph_properties: Option<StyleParagraphPropertiesContentStrict>,
    #[serde(rename = "section-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_properties: Option<StyleSectionPropertiesContentStrict>,
    #[serde(rename = "ruby-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby_properties: Option<StyleRubyPropertiesContentStrict>,
    #[serde(rename = "table-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_properties: Option<StyleTablePropertiesContentStrict>,
    #[serde(rename = "table-column-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_column_properties: Option<StyleTableColumnPropertiesContentStrict>,
    #[serde(rename = "table-row-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_row_properties: Option<StyleTableRowPropertiesContentStrict>,
    #[serde(rename = "table-cell-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_cell_properties: Option<StyleTableCellPropertiesContentStrict>,
    #[serde(rename = "graphic-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphic_properties: Option<StyleGraphicPropertiesContentStrict>,
    #[serde(rename = "drawing-page-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing_page_properties: Option<StyleDrawingPagePropertiesContentStrict>,
    #[serde(rename = "chart-properties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart_properties: Option<StyleChartPropertiesContentStrict>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleTabStop = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleTabStopAttlist {
    #[serde(rename = "@style:position")]
    pub position: Length,
    #[serde(rename = "@style:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@style:char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub char: Option<Character>,
    #[serde(rename = "@style:leader-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_type: Option<LineType>,
    #[serde(rename = "@style:leader-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_style: Option<LineStyle>,
    #[serde(rename = "@style:leader-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_width: Option<LineWidth>,
    #[serde(rename = "@style:leader-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_color: Option<String>,
    #[serde(rename = "@style:leader-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_text: Option<Character>,
    #[serde(rename = "@style:leader-text-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_text_style: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleTabStopCharAttlist {
    #[serde(rename = "@style:char")]
    pub char: Character,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTabStops {
    #[serde(rename = "tab-stops")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_stops: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleTableCellProperties = StyleTableCellPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableCellPropertiesAttlist {
    #[serde(rename = "@style:vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(rename = "@style:text-align-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align_source: Option<String>,
    #[serde(rename = "@style:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@style:glyph-orientation-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph_orientation_vertical: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:diagonal-tl-br")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_tl_br: Option<String>,
    #[serde(rename = "@style:diagonal-tl-br-widths")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_tl_br_widths: Option<BorderWidths>,
    #[serde(rename = "@style:diagonal-bl-tr")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_bl_tr: Option<String>,
    #[serde(rename = "@style:diagonal-bl-tr-widths")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_bl_tr_widths: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:wrap-option")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_option: Option<String>,
    #[serde(rename = "@style:rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_angle: Option<Angle>,
    #[serde(rename = "@style:rotation-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_align: Option<String>,
    #[serde(rename = "@style:cell-protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_protect: Option<String>,
    #[serde(rename = "@style:print-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_content: Option<Boolean>,
    #[serde(rename = "@style:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<NonNegativeInteger>,
    #[serde(rename = "@style:repeat-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_content: Option<Boolean>,
    #[serde(rename = "@style:shrink-to-fit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableCellPropertiesContentStrict {
    #[serde(rename = "@style:vertical-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(rename = "@style:text-align-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align_source: Option<String>,
    #[serde(rename = "@style:direction")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "@style:glyph-orientation-vertical")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph_orientation_vertical: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(rename = "@fo:border-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(rename = "@fo:border-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(rename = "@fo:border-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
    #[serde(rename = "@fo:border-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(rename = "@style:diagonal-tl-br")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_tl_br: Option<String>,
    #[serde(rename = "@style:diagonal-tl-br-widths")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_tl_br_widths: Option<BorderWidths>,
    #[serde(rename = "@style:diagonal-bl-tr")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_bl_tr: Option<String>,
    #[serde(rename = "@style:diagonal-bl-tr-widths")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_bl_tr_widths: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_top: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_bottom: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_left: Option<BorderWidths>,
    #[serde(rename = "@style:border-line-width-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_line_width_right: Option<BorderWidths>,
    #[serde(rename = "@fo:padding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<NonNegativeLength>,
    #[serde(rename = "@fo:padding-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<NonNegativeLength>,
    #[serde(rename = "@fo:wrap-option")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_option: Option<String>,
    #[serde(rename = "@style:rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_angle: Option<Angle>,
    #[serde(rename = "@style:rotation-align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_align: Option<String>,
    #[serde(rename = "@style:cell-protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_protect: Option<String>,
    #[serde(rename = "@style:print-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_content: Option<Boolean>,
    #[serde(rename = "@style:decimal-places")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<NonNegativeInteger>,
    #[serde(rename = "@style:repeat-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_content: Option<Boolean>,
    #[serde(rename = "@style:shrink-to-fit")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<Boolean>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableCellPropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleTableColumnProperties = StyleTableColumnPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableColumnPropertiesAttlist {
    #[serde(rename = "@style:column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_width: Option<PositiveLength>,
    #[serde(rename = "@style:rel-column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_column_width: Option<RelativeLength>,
    #[serde(rename = "@style:use-optimal-column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_optimal_column_width: Option<Boolean>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableColumnPropertiesContentStrict {
    #[serde(rename = "@style:column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_width: Option<PositiveLength>,
    #[serde(rename = "@style:rel-column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_column_width: Option<RelativeLength>,
    #[serde(rename = "@style:use-optimal-column-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_optimal_column_width: Option<Boolean>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableColumnPropertiesElements;

pub type StyleTableProperties = StyleTablePropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTablePropertiesAttlist {
    #[serde(rename = "@style:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PositiveLength>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<Percent>,
    #[serde(rename = "@table:align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@style:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:keep-with-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<String>,
    #[serde(rename = "@style:may-break-between-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub may_break_between_rows: Option<Boolean>,
    #[serde(rename = "@table:border-model")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_model: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@table:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    #[serde(rename = "@table:tab-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_color: Option<Color>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTablePropertiesContentStrict {
    #[serde(rename = "@style:width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PositiveLength>,
    #[serde(rename = "@style:rel-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_width: Option<Percent>,
    #[serde(rename = "@table:align")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(rename = "@fo:margin-left")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<String>,
    #[serde(rename = "@fo:margin-right")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<String>,
    #[serde(rename = "@fo:margin-top")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<String>,
    #[serde(rename = "@fo:margin-bottom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<String>,
    #[serde(rename = "@fo:margin")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(rename = "@style:page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowType>,
    #[serde(rename = "@fo:keep-with-next")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<String>,
    #[serde(rename = "@style:may-break-between-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub may_break_between_rows: Option<Boolean>,
    #[serde(rename = "@table:border-model")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_model: Option<String>,
    #[serde(rename = "@style:writing-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_mode: Option<String>,
    #[serde(rename = "@table:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    #[serde(rename = "@table:tab-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_color: Option<Color>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTablePropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleTableRowProperties = StyleTableRowPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableRowPropertiesAttlist {
    #[serde(rename = "@style:row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_height: Option<PositiveLength>,
    #[serde(rename = "@style:min-row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_row_height: Option<NonNegativeLength>,
    #[serde(rename = "@style:use-optimal-row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_optimal_row_height: Option<Boolean>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:keep-together")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_together: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableRowPropertiesContentStrict {
    #[serde(rename = "@style:row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_height: Option<PositiveLength>,
    #[serde(rename = "@style:min-row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_row_height: Option<NonNegativeLength>,
    #[serde(rename = "@style:use-optimal-row-height")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_optimal_row_height: Option<Boolean>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@fo:break-before")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_before: Option<String>,
    #[serde(rename = "@fo:break-after")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_after: Option<String>,
    #[serde(rename = "@fo:keep-together")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_together: Option<String>,
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTableRowPropertiesElements {
    #[serde(rename = "background-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type StyleTextProperties = StyleTextPropertiesContentStrict;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTextPropertiesAttlist {
    #[serde(rename = "@fo:font-variant")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(rename = "@fo:text-transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_transform: Option<String>,
    #[serde(rename = "@fo:color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(rename = "@style:use-window-font-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_window_font_color: Option<Boolean>,
    #[serde(rename = "@style:text-outline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_outline: Option<Boolean>,
    #[serde(rename = "@style:text-line-through-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_type: Option<LineType>,
    #[serde(rename = "@style:text-line-through-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_style: Option<LineStyle>,
    #[serde(rename = "@style:text-line-through-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_width: Option<LineWidth>,
    #[serde(rename = "@style:text-line-through-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_color: Option<String>,
    #[serde(rename = "@style:text-line-through-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_text: Option<String>,
    #[serde(rename = "@style:text-line-through-text-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_text_style: Option<StyleNameRef>,
    #[serde(rename = "@style:text-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_position: Option<String>,
    #[serde(rename = "@style:font-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(rename = "@style:font-name-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name_asian: Option<String>,
    #[serde(rename = "@style:font-name-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name_complex: Option<String>,
    #[serde(rename = "@fo:font-family")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(rename = "@style:font-family-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_asian: Option<String>,
    #[serde(rename = "@style:font-family-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_complex: Option<String>,
    #[serde(rename = "@style:font-family-generic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-family-generic-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic_asian: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-family-generic-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic_complex: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name: Option<String>,
    #[serde(rename = "@style:font-style-name-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name_asian: Option<String>,
    #[serde(rename = "@style:font-style-name-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name_complex: Option<String>,
    #[serde(rename = "@style:font-pitch")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch: Option<FontPitch>,
    #[serde(rename = "@style:font-pitch-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch_asian: Option<FontPitch>,
    #[serde(rename = "@style:font-pitch-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch_complex: Option<FontPitch>,
    #[serde(rename = "@style:font-charset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset: Option<TextEncoding>,
    #[serde(rename = "@style:font-charset-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset_asian: Option<TextEncoding>,
    #[serde(rename = "@style:font-charset-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset_complex: Option<TextEncoding>,
    #[serde(rename = "@fo:font-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,
    #[serde(rename = "@style:font-size-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_asian: Option<String>,
    #[serde(rename = "@style:font-size-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_complex: Option<String>,
    #[serde(rename = "@style:font-size-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel: Option<Length>,
    #[serde(rename = "@style:font-size-rel-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel_asian: Option<Length>,
    #[serde(rename = "@style:font-size-rel-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel_complex: Option<Length>,
    #[serde(rename = "@style:script-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_type: Option<String>,
    #[serde(rename = "@fo:letter-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_spacing: Option<String>,
    #[serde(rename = "@fo:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@style:language-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_asian: Option<LanguageCode>,
    #[serde(rename = "@style:language-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_complex: Option<LanguageCode>,
    #[serde(rename = "@fo:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@style:country-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_asian: Option<CountryCode>,
    #[serde(rename = "@style:country-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_complex: Option<CountryCode>,
    #[serde(rename = "@fo:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@style:script-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_asian: Option<ScriptCode>,
    #[serde(rename = "@style:script-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_complex: Option<ScriptCode>,
    #[serde(rename = "@style:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@style:rfc-language-tag-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag_asian: Option<Language>,
    #[serde(rename = "@style:rfc-language-tag-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag_complex: Option<Language>,
    #[serde(rename = "@fo:font-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(rename = "@style:font-style-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_asian: Option<FontStyle>,
    #[serde(rename = "@style:font-style-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_complex: Option<FontStyle>,
    #[serde(rename = "@style:font-relief")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_relief: Option<String>,
    #[serde(rename = "@fo:text-shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_shadow: Option<ShadowType>,
    #[serde(rename = "@style:text-underline-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_type: Option<LineType>,
    #[serde(rename = "@style:text-underline-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_style: Option<LineStyle>,
    #[serde(rename = "@style:text-underline-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_width: Option<LineWidth>,
    #[serde(rename = "@style:text-underline-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_color: Option<String>,
    #[serde(rename = "@style:text-overline-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_type: Option<LineType>,
    #[serde(rename = "@style:text-overline-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_style: Option<LineStyle>,
    #[serde(rename = "@style:text-overline-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_width: Option<LineWidth>,
    #[serde(rename = "@style:text-overline-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_color: Option<String>,
    #[serde(rename = "@style:text-overline-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_mode: Option<LineMode>,
    #[serde(rename = "@fo:font-weight")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(rename = "@style:font-weight-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight_asian: Option<FontWeight>,
    #[serde(rename = "@style:font-weight-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight_complex: Option<FontWeight>,
    #[serde(rename = "@style:text-underline-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_mode: Option<LineMode>,
    #[serde(rename = "@style:text-line-through-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_mode: Option<LineMode>,
    #[serde(rename = "@style:letter-kerning")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_kerning: Option<Boolean>,
    #[serde(rename = "@style:text-blinking")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_blinking: Option<Boolean>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:text-combine")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine: Option<String>,
    #[serde(rename = "@style:text-combine-start-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine_start_char: Option<Character>,
    #[serde(rename = "@style:text-combine-end-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine_end_char: Option<Character>,
    #[serde(rename = "@style:text-emphasize")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_emphasize: Option<String>,
    #[serde(rename = "@style:text-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<Percent>,
    #[serde(rename = "@style:text-rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotation_angle: Option<Angle>,
    #[serde(rename = "@style:text-rotation-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotation_scale: Option<String>,
    #[serde(rename = "@fo:hyphenate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenate: Option<Boolean>,
    #[serde(rename = "@fo:hyphenation-remain-char-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_remain_char_count: Option<PositiveInteger>,
    #[serde(rename = "@fo:hyphenation-push-char-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_push_char_count: Option<PositiveInteger>,
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "@text:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTextPropertiesContentStrict {
    #[serde(rename = "@fo:font-variant")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(rename = "@fo:text-transform")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_transform: Option<String>,
    #[serde(rename = "@fo:color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(rename = "@style:use-window-font-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_window_font_color: Option<Boolean>,
    #[serde(rename = "@style:text-outline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_outline: Option<Boolean>,
    #[serde(rename = "@style:text-line-through-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_type: Option<LineType>,
    #[serde(rename = "@style:text-line-through-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_style: Option<LineStyle>,
    #[serde(rename = "@style:text-line-through-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_width: Option<LineWidth>,
    #[serde(rename = "@style:text-line-through-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_color: Option<String>,
    #[serde(rename = "@style:text-line-through-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_text: Option<String>,
    #[serde(rename = "@style:text-line-through-text-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_text_style: Option<StyleNameRef>,
    #[serde(rename = "@style:text-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_position: Option<String>,
    #[serde(rename = "@style:font-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(rename = "@style:font-name-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name_asian: Option<String>,
    #[serde(rename = "@style:font-name-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name_complex: Option<String>,
    #[serde(rename = "@fo:font-family")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(rename = "@style:font-family-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_asian: Option<String>,
    #[serde(rename = "@style:font-family-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_complex: Option<String>,
    #[serde(rename = "@style:font-family-generic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-family-generic-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic_asian: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-family-generic-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family_generic_complex: Option<FontFamilyGeneric>,
    #[serde(rename = "@style:font-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name: Option<String>,
    #[serde(rename = "@style:font-style-name-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name_asian: Option<String>,
    #[serde(rename = "@style:font-style-name-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_name_complex: Option<String>,
    #[serde(rename = "@style:font-pitch")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch: Option<FontPitch>,
    #[serde(rename = "@style:font-pitch-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch_asian: Option<FontPitch>,
    #[serde(rename = "@style:font-pitch-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_pitch_complex: Option<FontPitch>,
    #[serde(rename = "@style:font-charset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset: Option<TextEncoding>,
    #[serde(rename = "@style:font-charset-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset_asian: Option<TextEncoding>,
    #[serde(rename = "@style:font-charset-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_charset_complex: Option<TextEncoding>,
    #[serde(rename = "@fo:font-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,
    #[serde(rename = "@style:font-size-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_asian: Option<String>,
    #[serde(rename = "@style:font-size-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_complex: Option<String>,
    #[serde(rename = "@style:font-size-rel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel: Option<Length>,
    #[serde(rename = "@style:font-size-rel-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel_asian: Option<Length>,
    #[serde(rename = "@style:font-size-rel-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_rel_complex: Option<Length>,
    #[serde(rename = "@style:script-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_type: Option<String>,
    #[serde(rename = "@fo:letter-spacing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_spacing: Option<String>,
    #[serde(rename = "@fo:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@style:language-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_asian: Option<LanguageCode>,
    #[serde(rename = "@style:language-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_complex: Option<LanguageCode>,
    #[serde(rename = "@fo:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@style:country-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_asian: Option<CountryCode>,
    #[serde(rename = "@style:country-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_complex: Option<CountryCode>,
    #[serde(rename = "@fo:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@style:script-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_asian: Option<ScriptCode>,
    #[serde(rename = "@style:script-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_complex: Option<ScriptCode>,
    #[serde(rename = "@style:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@style:rfc-language-tag-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag_asian: Option<Language>,
    #[serde(rename = "@style:rfc-language-tag-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag_complex: Option<Language>,
    #[serde(rename = "@fo:font-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(rename = "@style:font-style-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_asian: Option<FontStyle>,
    #[serde(rename = "@style:font-style-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style_complex: Option<FontStyle>,
    #[serde(rename = "@style:font-relief")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_relief: Option<String>,
    #[serde(rename = "@fo:text-shadow")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_shadow: Option<ShadowType>,
    #[serde(rename = "@style:text-underline-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_type: Option<LineType>,
    #[serde(rename = "@style:text-underline-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_style: Option<LineStyle>,
    #[serde(rename = "@style:text-underline-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_width: Option<LineWidth>,
    #[serde(rename = "@style:text-underline-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_color: Option<String>,
    #[serde(rename = "@style:text-overline-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_type: Option<LineType>,
    #[serde(rename = "@style:text-overline-style")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_style: Option<LineStyle>,
    #[serde(rename = "@style:text-overline-width")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_width: Option<LineWidth>,
    #[serde(rename = "@style:text-overline-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_color: Option<String>,
    #[serde(rename = "@style:text-overline-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_overline_mode: Option<LineMode>,
    #[serde(rename = "@fo:font-weight")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(rename = "@style:font-weight-asian")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight_asian: Option<FontWeight>,
    #[serde(rename = "@style:font-weight-complex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight_complex: Option<FontWeight>,
    #[serde(rename = "@style:text-underline-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_underline_mode: Option<LineMode>,
    #[serde(rename = "@style:text-line-through-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_through_mode: Option<LineMode>,
    #[serde(rename = "@style:letter-kerning")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_kerning: Option<Boolean>,
    #[serde(rename = "@style:text-blinking")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_blinking: Option<Boolean>,
    #[serde(rename = "@fo:background-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(rename = "@style:text-combine")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine: Option<String>,
    #[serde(rename = "@style:text-combine-start-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine_start_char: Option<Character>,
    #[serde(rename = "@style:text-combine-end-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_combine_end_char: Option<Character>,
    #[serde(rename = "@style:text-emphasize")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_emphasize: Option<String>,
    #[serde(rename = "@style:text-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<Percent>,
    #[serde(rename = "@style:text-rotation-angle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotation_angle: Option<Angle>,
    #[serde(rename = "@style:text-rotation-scale")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotation_scale: Option<String>,
    #[serde(rename = "@fo:hyphenate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenate: Option<Boolean>,
    #[serde(rename = "@fo:hyphenation-remain-char-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_remain_char_count: Option<PositiveInteger>,
    #[serde(rename = "@fo:hyphenation-push-char-count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyphenation_push_char_count: Option<PositiveInteger>,
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "@text:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleTextPropertiesElements;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleNameRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Styles {
    #[serde(rename = "style")]
    pub style: String,
    #[serde(rename = "list-style")]
    pub list_style: String,
    #[serde(rename = "number-style")]
    pub number_style: String,
    #[serde(rename = "currency-style")]
    pub currency_style: String,
    #[serde(rename = "percentage-style")]
    pub percentage_style: String,
    #[serde(rename = "date-style")]
    pub date_style: String,
    #[serde(rename = "time-style")]
    pub time_style: String,
    #[serde(rename = "boolean-style")]
    pub boolean_style: String,
    #[serde(rename = "text-style")]
    pub text_style: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type SvgDefinitionSrc = String;

pub type SvgDesc = String;

pub type SvgFontFaceFormat = String;

pub type SvgFontFaceName = String;

pub type SvgFontFaceSrc = String;

pub type SvgFontFaceUri = String;

pub type SvgLinearGradient = String;

pub type SvgRadialGradient = String;

pub type SvgStop = String;

pub type SvgTitle = String;

pub type TableBackground = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBackgroundAttlist {
    #[serde(rename = "@table:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableBody = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableCalculationSettingAttlist {
    #[serde(rename = "@table:case-sensitive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<Boolean>,
    #[serde(rename = "@table:precision-as-shown")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision_as_shown: Option<Boolean>,
    #[serde(rename = "@table:search-criteria-must-apply-to-whole-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_criteria_must_apply_to_whole_cell: Option<Boolean>,
    #[serde(rename = "@table:automatic-find-labels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_find_labels: Option<Boolean>,
    #[serde(rename = "@table:use-regular-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_regular_expressions: Option<Boolean>,
    #[serde(rename = "@table:use-wildcards")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_wildcards: Option<Boolean>,
    #[serde(rename = "@table:null-year")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_year: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableCalculationSettings = String;

pub type TableCellAddress = String;

pub type TableCellContentChange = String;

pub type TableCellContentDeletion = String;

pub type TableCellRangeSource = String;

pub type TableChangeDeletion = String;

pub type TableChangeTrackTableCell = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableChangeTrackTableCellAttlist {
    #[serde(rename = "@table:cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_address: Option<CellAddress>,
    #[serde(rename = "@table:matrix-covered")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_covered: Option<Boolean>,
    #[serde(rename = "@table:formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(rename = "@table:number-matrix-columns-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_matrix_columns_spanned: Option<PositiveInteger>,
    #[serde(rename = "@table:number-matrix-rows-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_matrix_rows_spanned: Option<PositiveInteger>,
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "@office:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@office:currency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "@office:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@office:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<Duration>,
    #[serde(rename = "@office:boolean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<Boolean>,
    #[serde(rename = "@office:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumns {
    #[serde(rename = "table-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_columns: Option<String>,
    #[serde(rename = "table-column")]
    pub table_column: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableColumnsAndGroups {
    #[serde(rename = "table-column-group")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_column_group: Vec<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumnsNoGroup {
    #[serde(rename = "table-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_columns: Option<String>,
    #[serde(rename = "table-column")]
    pub table_column: String,
    #[serde(rename = "table-header-columns")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_header_columns: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TableConsolidation = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConsolidationAttlist {
    #[serde(rename = "@table:function")]
    pub function: String,
    #[serde(rename = "@table:source-cell-range-addresses")]
    pub source_cell_range_addresses: CellRangeAddressList,
    #[serde(rename = "@table:target-cell-address")]
    pub target_cell_address: CellAddress,
    #[serde(rename = "@table:use-labels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_labels: Option<String>,
    #[serde(rename = "@table:link-to-source-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_to_source_data: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableContentValidation = String;

pub type TableContentValidations = String;

pub type TableCoveredTableCell = String;

pub type TableCutOffs = String;

pub type TableDataPilotDisplayInfo = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotDisplayInfoAttlist {
    #[serde(rename = "@table:enabled")]
    pub enabled: Boolean,
    #[serde(rename = "@table:data-field")]
    pub data_field: String,
    #[serde(rename = "@table:member-count")]
    pub member_count: NonNegativeInteger,
    #[serde(rename = "@table:display-member-mode")]
    pub display_member_mode: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotField = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotFieldAttlist {
    #[serde(rename = "@table:source-field-name")]
    pub source_field_name: String,
    #[serde(rename = "@table:orientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(rename = "@table:selected-page")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_page: Option<String>,
    #[serde(rename = "@table:is-data-layout-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_data_layout_field: Option<String>,
    #[serde(rename = "@table:function")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(rename = "@table:used-hierarchy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_hierarchy: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotFieldReference = TableDataPilotFieldReferenceAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotFieldReferenceAttlist {
    #[serde(rename = "@table:field-name")]
    pub field_name: String,
    #[serde(rename = "@table:member-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_type: Option<String>,
    #[serde(rename = "@table:member-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    #[serde(rename = "@table:type")]
    pub r#type: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotGroup = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotGroupAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotGroupMember = TableDataPilotGroupMemberAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotGroupMemberAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotGroups = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotGroupsAttlist {
    #[serde(rename = "@table:source-field-name")]
    pub source_field_name: String,
    #[serde(rename = "@table:date-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_start: Option<String>,
    #[serde(rename = "@table:start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(rename = "@table:date-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_end: Option<String>,
    #[serde(rename = "@table:end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(rename = "@table:step")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Double>,
    #[serde(rename = "@table:grouped-by")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouped_by: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotLayoutInfo = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotLayoutInfoAttlist {
    #[serde(rename = "@table:layout-mode")]
    pub layout_mode: String,
    #[serde(rename = "@table:add-empty-lines")]
    pub add_empty_lines: Boolean,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotLevel = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableDataPilotLevelAttlist {
    #[serde(rename = "@table:show-empty")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_empty: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotMember = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotMemberAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    #[serde(rename = "@table:show-details")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_details: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotMembers = String;

pub type TableDataPilotSortInfo = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotSortInfoAttlist {
    #[serde(rename = "@table:sort-mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_mode: Option<String>,
    #[serde(rename = "@table:data-field")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(rename = "@table:order")]
    pub order: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotSubtotal = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotSubtotalAttlist {
    #[serde(rename = "@table:function")]
    pub function: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotSubtotals = String;

pub type TableDataPilotTable = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataPilotTableAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:application-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_data: Option<String>,
    #[serde(rename = "@table:grand-total")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grand_total: Option<String>,
    #[serde(rename = "@table:ignore-empty-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_empty_rows: Option<Boolean>,
    #[serde(rename = "@table:identify-categories")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identify_categories: Option<Boolean>,
    #[serde(rename = "@table:target-range-address")]
    pub target_range_address: CellRangeAddress,
    #[serde(rename = "@table:buttons")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buttons: Option<CellRangeAddressList>,
    #[serde(rename = "@table:show-filter-button")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_filter_button: Option<Boolean>,
    #[serde(rename = "@table:drill-down-on-double-click")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill_down_on_double_click: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDataPilotTables = String;

pub type TableDatabaseRange = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDatabaseRangeAttlist {
    #[serde(rename = "@table:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@table:is-selection")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_selection: Option<Boolean>,
    #[serde(rename = "@table:on-update-keep-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_update_keep_styles: Option<Boolean>,
    #[serde(rename = "@table:on-update-keep-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_update_keep_size: Option<Boolean>,
    #[serde(rename = "@table:has-persistent-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_persistent_data: Option<Boolean>,
    #[serde(rename = "@table:orientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(rename = "@table:contains-header")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_header: Option<Boolean>,
    #[serde(rename = "@table:display-filter-buttons")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_filter_buttons: Option<Boolean>,
    #[serde(rename = "@table:target-range-address")]
    pub target_range_address: CellRangeAddress,
    #[serde(rename = "@table:refresh-delay")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_delay: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDatabaseRanges = String;

pub type TableDatabaseSourceQuery = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDatabaseSourceQueryAttlist {
    #[serde(rename = "@table:database-name")]
    pub database_name: String,
    #[serde(rename = "@table:query-name")]
    pub query_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDatabaseSourceSql = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDatabaseSourceSqlAttlist {
    #[serde(rename = "@table:database-name")]
    pub database_name: String,
    #[serde(rename = "@table:sql-statement")]
    pub sql_statement: String,
    #[serde(rename = "@table:parse-sql-statement")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_sql_statement: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDatabaseSourceTable = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDatabaseSourceTableAttlist {
    #[serde(rename = "@table:database-name")]
    pub database_name: String,
    #[serde(rename = "@table:database-table-name")]
    pub database_table_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDdeLink = String;

pub type TableDdeLinks = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableDecls {
    #[serde(rename = "calculation-settings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_settings: Option<String>,
    #[serde(rename = "content-validations")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validations: Option<String>,
    #[serde(rename = "label-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ranges: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TableDeletion = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDeletionAttlist {
    #[serde(rename = "@table:type")]
    pub r#type: String,
    #[serde(rename = "@table:position")]
    pub position: Integer,
    #[serde(rename = "@table:table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<Integer>,
    #[serde(rename = "@table:multi-deletion-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_deletion_spanned: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableDeletions = String;

pub type TableDependencies = String;

pub type TableDependency = String;

pub type TableDesc = String;

pub type TableDetective = String;

pub type TableErrorMacro = String;

pub type TableErrorMessage = String;

pub type TableEvenColumns = String;

pub type TableEvenRows = String;

pub type TableFilter = String;

pub type TableFilterAnd = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableFilterAttlist {
    #[serde(rename = "@table:target-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_range_address: Option<CellRangeAddress>,
    #[serde(rename = "@table:condition-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_source: Option<String>,
    #[serde(rename = "@table:condition-source-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_source_range_address: Option<CellRangeAddress>,
    #[serde(rename = "@table:display-duplicates")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_duplicates: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableFilterCondition = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFilterConditionAttlist {
    #[serde(rename = "@table:field-number")]
    pub field_number: NonNegativeInteger,
    #[serde(rename = "@table:value")]
    pub value: String,
    #[serde(rename = "@table:operator")]
    pub operator: String,
    #[serde(rename = "@table:case-sensitive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<String>,
    #[serde(rename = "@table:data-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableFilterOr = String;

pub type TableFilterSetItem = String;

pub type TableFirstColumn = String;

pub type TableFirstRow = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableFunctions {
    #[serde(rename = "named-expressions")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_expressions: Option<String>,
    #[serde(rename = "database-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_ranges: Option<String>,
    #[serde(rename = "data-pilot-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pilot_tables: Option<String>,
    #[serde(rename = "consolidation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<String>,
    #[serde(rename = "dde-links")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_links: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TableHelpMessage = String;

pub type TableHighlightedRange = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableHighlightedRangeAttlist {
    #[serde(rename = "@table:cell-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_range_address: Option<CellRangeAddress>,
    #[serde(rename = "@table:direction")]
    pub direction: String,
    #[serde(rename = "@table:contains-error")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_error: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableHighlightedRangeAttlistInvalid {
    #[serde(rename = "@table:marked-invalid")]
    pub marked_invalid: Boolean,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableInsertion = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInsertionAttlist {
    #[serde(rename = "@table:type")]
    pub r#type: String,
    #[serde(rename = "@table:position")]
    pub position: Integer,
    #[serde(rename = "@table:count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<PositiveInteger>,
    #[serde(rename = "@table:table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableInsertionCutOff = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInsertionCutOffAttlist {
    #[serde(rename = "@table:id")]
    pub id: String,
    #[serde(rename = "@table:position")]
    pub position: Integer,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableIteration = String;

pub type TableLabelRange = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableLabelRangeAttlist {
    #[serde(rename = "@table:label-cell-range-address")]
    pub label_cell_range_address: CellRangeAddress,
    #[serde(rename = "@table:data-cell-range-address")]
    pub data_cell_range_address: CellRangeAddress,
    #[serde(rename = "@table:orientation")]
    pub orientation: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableLabelRanges = String;

pub type TableLastColumn = String;

pub type TableLastRow = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableLinkedSourceAttlist {
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@table:filter-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_name: Option<String>,
    #[serde(rename = "@table:filter-options")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_options: Option<String>,
    #[serde(rename = "@table:refresh-delay")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_delay: Option<Duration>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableMovement = String;

pub type TableMovementCutOff = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableMovementCutOffAttlist {
    #[serde(rename = "@table:position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Integer>,
    #[serde(rename = "@table:start-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_position: Option<Integer>,
    #[serde(rename = "@table:end-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_position: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableNamedExpression = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableNamedExpressionAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:expression")]
    pub expression: String,
    #[serde(rename = "@table:base-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_cell_address: Option<CellAddress>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableNamedExpressions = String;

pub type TableNamedRange = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableNamedRangeAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:cell-range-address")]
    pub cell_range_address: CellRangeAddress,
    #[serde(rename = "@table:base-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_cell_address: Option<CellAddress>,
    #[serde(rename = "@table:range-usable-as")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_usable_as: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableNullDate = String;

pub type TableOddColumns = String;

pub type TableOddRows = String;

pub type TableOperation = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableOperationAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:index")]
    pub index: NonNegativeInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TablePrevious = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRows {
    #[serde(rename = "table-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_rows: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-row")]
    pub table_row: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableRowsAndGroups {
    #[serde(rename = "table-row-group")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_row_group: Vec<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRowsNoGroup {
    #[serde(rename = "table-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_rows: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-row")]
    pub table_row: String,
    #[serde(rename = "table-header-rows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_header_rows: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TableScenario = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableScenarioAttlist {
    #[serde(rename = "@table:scenario-ranges")]
    pub scenario_ranges: CellRangeAddressList,
    #[serde(rename = "@table:is-active")]
    pub is_active: Boolean,
    #[serde(rename = "@table:display-border")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_border: Option<Boolean>,
    #[serde(rename = "@table:border-color")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Color>,
    #[serde(rename = "@table:copy-back")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_back: Option<Boolean>,
    #[serde(rename = "@table:copy-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_styles: Option<Boolean>,
    #[serde(rename = "@table:copy-formulas")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_formulas: Option<Boolean>,
    #[serde(rename = "@table:comment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "@table:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableShapes = String;

pub type TableSort = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableSortAttlist {
    #[serde(rename = "@table:bind-styles-to-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_styles_to_content: Option<Boolean>,
    #[serde(rename = "@table:target-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_range_address: Option<CellRangeAddress>,
    #[serde(rename = "@table:case-sensitive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<Boolean>,
    #[serde(rename = "@table:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@table:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@table:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@table:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@table:algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    #[serde(rename = "@table:embedded-number-behavior")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedded_number_behavior: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSortBy = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSortByAttlist {
    #[serde(rename = "@table:field-number")]
    pub field_number: NonNegativeInteger,
    #[serde(rename = "@table:data-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    #[serde(rename = "@table:order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSortGroups = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableSortGroupsAttlist {
    #[serde(rename = "@table:data-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    #[serde(rename = "@table:order")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSourceCellRange = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableSourceCellRangeAttlist {
    #[serde(rename = "@table:cell-range-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_range_address: Option<CellRangeAddress>,
    #[serde(rename = "@table:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSourceRangeAddress = String;

pub type TableSourceService = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSourceServiceAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:source-name")]
    pub source_name: String,
    #[serde(rename = "@table:object-name")]
    pub object_name: String,
    #[serde(rename = "@table:user-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(rename = "@table:password")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSubtotalField = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSubtotalFieldAttlist {
    #[serde(rename = "@table:field-number")]
    pub field_number: NonNegativeInteger,
    #[serde(rename = "@table:function")]
    pub function: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSubtotalRule = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSubtotalRuleAttlist {
    #[serde(rename = "@table:group-by-field-number")]
    pub group_by_field_number: NonNegativeInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableSubtotalRules = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableSubtotalRulesAttlist {
    #[serde(rename = "@table:bind-styles-to-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_styles_to_content: Option<Boolean>,
    #[serde(rename = "@table:case-sensitive")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<Boolean>,
    #[serde(rename = "@table:page-breaks-on-group-change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_breaks_on_group_change: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTable = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableAttlist {
    #[serde(rename = "@table:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@table:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:template-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(rename = "@table:use-first-row-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_first_row_styles: Option<Boolean>,
    #[serde(rename = "@table:use-last-row-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_last_row_styles: Option<Boolean>,
    #[serde(rename = "@table:use-first-column-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_first_column_styles: Option<Boolean>,
    #[serde(rename = "@table:use-last-column-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_last_column_styles: Option<Boolean>,
    #[serde(rename = "@table:use-banding-rows-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_banding_rows_styles: Option<Boolean>,
    #[serde(rename = "@table:use-banding-columns-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_banding_columns_styles: Option<Boolean>,
    #[serde(rename = "@table:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    #[serde(rename = "@table:protection-key")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key: Option<String>,
    #[serde(rename = "@table:protection-key-digest-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key_digest_algorithm: Option<AnyIRI>,
    #[serde(rename = "@table:print")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<Boolean>,
    #[serde(rename = "@table:print-ranges")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_ranges: Option<CellRangeAddressList>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@table:is-sub-table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_sub_table: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableCell = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableCellAttlist {
    #[serde(rename = "@table:number-columns-repeated")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_columns_repeated: Option<PositiveInteger>,
    #[serde(rename = "@table:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:content-validation-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_validation_name: Option<String>,
    #[serde(rename = "@table:formula")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(rename = "@office:value-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(rename = "@office:value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Double>,
    #[serde(rename = "@office:currency")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "@office:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@office:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<Duration>,
    #[serde(rename = "@office:boolean-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<Boolean>,
    #[serde(rename = "@office:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    #[serde(rename = "@table:protect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<Boolean>,
    #[serde(rename = "@table:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@xhtml:about")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<URIorSafeCURIE>,
    #[serde(rename = "@xhtml:property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<CURIEs>,
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableCellAttlistExtra {
    #[serde(rename = "@table:number-columns-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_columns_spanned: Option<PositiveInteger>,
    #[serde(rename = "@table:number-rows-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_rows_spanned: Option<PositiveInteger>,
    #[serde(rename = "@table:number-matrix-columns-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_matrix_columns_spanned: Option<PositiveInteger>,
    #[serde(rename = "@table:number-matrix-rows-spanned")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_matrix_rows_spanned: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableCellContent {
    #[serde(rename = "cell-range-source")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_range_source: Option<String>,
    #[serde(rename = "annotation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    #[serde(rename = "detective")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detective: Option<String>,
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    #[serde(rename = "numbered-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_paragraph: Option<String>,
    #[serde(rename = "table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "section")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-of-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_of_content: Option<String>,
    #[serde(rename = "illustration-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration_index: Option<String>,
    #[serde(rename = "table-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_index: Option<String>,
    #[serde(rename = "object-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<String>,
    #[serde(rename = "user-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index: Option<String>,
    #[serde(rename = "alphabetical-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index: Option<String>,
    #[serde(rename = "bibliography")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<String>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableTableCellRangeSourceAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:last-column-spanned")]
    pub last_column_spanned: PositiveInteger,
    #[serde(rename = "@table:last-row-spanned")]
    pub last_row_spanned: PositiveInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableColumn = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableColumnAttlist {
    #[serde(rename = "@table:number-columns-repeated")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_columns_repeated: Option<PositiveInteger>,
    #[serde(rename = "@table:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:visibility")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<TableVisibilityValue>,
    #[serde(rename = "@table:default-cell-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_cell_style_name: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableColumnGroup = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableColumnGroupAttlist {
    #[serde(rename = "@table:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableColumns = String;

pub type TableTableHeaderColumns = String;

pub type TableTableHeaderRows = String;

pub type TableTableRow = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableRowAttlist {
    #[serde(rename = "@table:number-rows-repeated")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_rows_repeated: Option<PositiveInteger>,
    #[serde(rename = "@table:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:default-cell-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_cell_style_name: Option<StyleNameRef>,
    #[serde(rename = "@table:visibility")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<TableVisibilityValue>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableRowGroup = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableRowGroupAttlist {
    #[serde(rename = "@table:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableRows = String;

pub type TableTableSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTableSourceAttlist {
    #[serde(rename = "@table:mode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(rename = "@table:table-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTableTemplate = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableTableTemplateAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:first-row-start-column")]
    pub first_row_start_column: RowOrCol,
    #[serde(rename = "@table:first-row-end-column")]
    pub first_row_end_column: RowOrCol,
    #[serde(rename = "@table:last-row-start-column")]
    pub last_row_start_column: RowOrCol,
    #[serde(rename = "@table:last-row-end-column")]
    pub last_row_end_column: RowOrCol,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TableTargetRangeAddress = String;

pub type TableTitle = String;

pub type TableTrackedChanges = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableTrackedChangesAttlist {
    #[serde(rename = "@table:track-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_changes: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableValidationAttlist {
    #[serde(rename = "@table:name")]
    pub name: String,
    #[serde(rename = "@table:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(rename = "@table:base-cell-address")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_cell_address: Option<CellAddress>,
    #[serde(rename = "@table:allow-empty-cell")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_empty_cell: Option<Boolean>,
    #[serde(rename = "@table:display-list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_list: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetFrame {
    #[serde(rename = "@office:target-frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame: Option<TargetFrameName>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetLocation {
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetFrameName {
    #[serde(rename = "$text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextA = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAAttlist {
    #[serde(rename = "@office:name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@office:title")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "@xlink:type")]
    pub r#type: String,
    #[serde(rename = "@xlink:href")]
    pub href: AnyIRI,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "@office:target-frame-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame_name: Option<TargetFrameName>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:visited-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visited_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextAlphabeticalIndex = String;

pub type TextAlphabeticalIndexAutoMarkFile = String;

pub type TextAlphabeticalIndexEntryTemplate = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAlphabeticalIndexEntryTemplateAttrs {
    #[serde(rename = "@text:outline-level")]
    pub outline_level: String,
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextAlphabeticalIndexMarkAttrs {
    #[serde(rename = "@text:key1")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key1: Option<String>,
    #[serde(rename = "@text:key2")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key2: Option<String>,
    #[serde(rename = "@text:string-value-phonetic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value_phonetic: Option<String>,
    #[serde(rename = "@text:key1-phonetic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key1_phonetic: Option<String>,
    #[serde(rename = "@text:key2-phonetic")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key2_phonetic: Option<String>,
    #[serde(rename = "@text:main-entry")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_entry: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextAlphabeticalIndexSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextAlphabeticalIndexSourceAttrs {
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    #[serde(rename = "@text:ignore-case")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_case: Option<Boolean>,
    #[serde(rename = "@text:main-entry-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_entry_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:alphabetical-separators")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_separators: Option<Boolean>,
    #[serde(rename = "@text:combine-entries")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combine_entries: Option<Boolean>,
    #[serde(rename = "@text:combine-entries-with-dash")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combine_entries_with_dash: Option<Boolean>,
    #[serde(rename = "@text:combine-entries-with-pp")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combine_entries_with_pp: Option<Boolean>,
    #[serde(rename = "@text:use-keys-as-entries")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_keys_as_entries: Option<Boolean>,
    #[serde(rename = "@text:capitalize-entries")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capitalize_entries: Option<Boolean>,
    #[serde(rename = "@text:comma-separated")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comma_separated: Option<Boolean>,
    #[serde(rename = "@fo:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@fo:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@fo:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@style:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@text:sort-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_algorithm: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextBibliography = String;

pub type TextBibliographyConfiguration = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextBibliographyConfigurationAttlist {
    #[serde(rename = "@text:prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(rename = "@text:suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(rename = "@text:numbered-entries")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_entries: Option<Boolean>,
    #[serde(rename = "@text:sort-by-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by_position: Option<Boolean>,
    #[serde(rename = "@fo:language")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageCode>,
    #[serde(rename = "@fo:country")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    #[serde(rename = "@fo:script")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptCode>,
    #[serde(rename = "@style:rfc-language-tag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfc_language_tag: Option<Language>,
    #[serde(rename = "@text:sort-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_algorithm: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextBibliographyEntryTemplate = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBibliographyEntryTemplateAttrs {
    #[serde(rename = "@text:bibliography-type")]
    pub bibliography_type: TextBibliographyTypes,
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextBibliographySource = String;

pub type TextBookmark = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBookmarkAttlist {
    #[serde(rename = "@text:name")]
    pub name: String,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextBookmarkEnd = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBookmarkEndAttlist {
    #[serde(rename = "@text:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextBookmarkRefContent {
    #[serde(rename = "@text:reference-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_format: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextBookmarkStart = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBookmarkStartAttlist {
    #[serde(rename = "@text:name")]
    pub name: String,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@xhtml:about")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<URIorSafeCURIE>,
    #[serde(rename = "@xhtml:property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<CURIEs>,
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextChangedRegion = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChangedRegionAttr {
    #[serde(rename = "@xml:id")]
    pub id: ID,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextChangedRegionContent {
    #[serde(rename = "insertion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertion: Option<OfficeChangeInfo>,
    #[serde(rename = "deletion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion: Option<String>,
    #[serde(rename = "format-change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_change: Option<OfficeChangeInfo>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChapterAttlist {
    #[serde(rename = "@text:display")]
    pub display: String,
    #[serde(rename = "@text:outline-level")]
    pub outline_level: NonNegativeInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextCommonRefContent {
    #[serde(rename = "@text:ref-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConditionalTextAttlist {
    #[serde(rename = "@text:condition")]
    pub condition: String,
    #[serde(rename = "@text:string-value-if-true")]
    pub string_value_if_true: String,
    #[serde(rename = "@text:string-value-if-false")]
    pub string_value_if_false: String,
    #[serde(rename = "@text:current-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    #[serde(rename = "numbered-paragraph")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_paragraph: Option<String>,
    #[serde(rename = "table")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "section")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_page_break: Option<()>,
    #[serde(rename = "table-of-content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_of_content: Option<String>,
    #[serde(rename = "illustration-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration_index: Option<String>,
    #[serde(rename = "table-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_index: Option<String>,
    #[serde(rename = "object-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<String>,
    #[serde(rename = "user-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_index: Option<String>,
    #[serde(rename = "alphabetical-index")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index: Option<String>,
    #[serde(rename = "bibliography")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<String>,
    #[serde(rename = "rect")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<String>,
    #[serde(rename = "line")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(rename = "polyline")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
    #[serde(rename = "polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<String>,
    #[serde(rename = "regular-polygon")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_polygon: Option<String>,
    #[serde(rename = "path")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "circle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<String>,
    #[serde(rename = "ellipse")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<String>,
    #[serde(rename = "g")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(rename = "page-thumbnail")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_thumbnail: Option<String>,
    #[serde(rename = "frame")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(rename = "measure")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(rename = "caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(rename = "connector")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(rename = "control")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<String>,
    #[serde(rename = "scene")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "custom-shape")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_shape: Option<String>,
    #[serde(rename = "a")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<String>,
    #[serde(rename = "change")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ChangeMarkAttr>,
    #[serde(rename = "change-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_start: Option<ChangeMarkAttr>,
    #[serde(rename = "change-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_end: Option<ChangeMarkAttr>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDatabaseDisplayAttlist {
    #[serde(rename = "@text:table-name")]
    pub table_name: String,
    #[serde(rename = "@text:table-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_type: Option<String>,
    #[serde(rename = "@text:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "connection-resource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_resource: Option<String>,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:column-name")]
    pub column_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDatabaseNextAttlist {
    #[serde(rename = "@text:table-name")]
    pub table_name: String,
    #[serde(rename = "@text:table-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_type: Option<String>,
    #[serde(rename = "@text:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "connection-resource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_resource: Option<String>,
    #[serde(rename = "@text:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDatabaseRowSelectAttlist {
    #[serde(rename = "@text:table-name")]
    pub table_name: String,
    #[serde(rename = "@text:table-type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_type: Option<String>,
    #[serde(rename = "@text:database-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "connection-resource")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_resource: Option<String>,
    #[serde(rename = "@text:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(rename = "@text:row-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_number: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextDateAttlist {
    #[serde(rename = "@text:fixed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Boolean>,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:date-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_value: Option<DateOrDateTime>,
    #[serde(rename = "@text:date-adjust")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_adjust: Option<Duration>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextDdeConnectionDecl = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDdeConnectionDeclAttlist {
    #[serde(rename = "@office:name")]
    pub name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextDecls {
    #[serde(rename = "variable-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_decls: Option<String>,
    #[serde(rename = "sequence-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_decls: Option<String>,
    #[serde(rename = "user-field-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_field_decls: Option<String>,
    #[serde(rename = "dde-connection-decls")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dde_connection_decls: Option<String>,
    #[serde(rename = "alphabetical-index-auto-mark-file")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabetical_index_auto_mark_file: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextDropDown = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextFileNameAttlist {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "@text:fixed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextGetPageVariableAttlist {
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextH = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextHiddenParagraphAttlist {
    #[serde(rename = "@text:condition")]
    pub condition: String,
    #[serde(rename = "@text:is-hidden")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_hidden: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextHiddenTextAttlist {
    #[serde(rename = "@text:condition")]
    pub condition: String,
    #[serde(rename = "@text:string-value")]
    pub string_value: String,
    #[serde(rename = "@text:is-hidden")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_hidden: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextId {
    #[serde(rename = "@text:id")]
    pub id: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIllustrationIndex = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextIllustrationIndexEntryContent {
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    #[serde(rename = "index-entry-chapter")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_chapter: Vec<String>,
    #[serde(rename = "index-entry-page-number")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_page_number: Vec<String>,
    #[serde(rename = "index-entry-text")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_text: Vec<String>,
    #[serde(rename = "index-entry-span")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_span: Vec<String>,
    #[serde(rename = "index-entry-tab-stop")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_tab_stop: Vec<String>,
    #[serde(rename = "index-entry-link-start")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_link_start: Vec<String>,
    #[serde(rename = "index-entry-link-end")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_entry_link_end: Vec<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextIllustrationIndexEntryTemplate = TextIllustrationIndexEntryContent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextIllustrationIndexEntryTemplateAttrs {
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIllustrationIndexSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextIllustrationIndexSourceAttrs {
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    #[serde(rename = "@text:use-caption")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_caption: Option<Boolean>,
    #[serde(rename = "@text:caption-sequence-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_sequence_name: Option<String>,
    #[serde(rename = "@text:caption-sequence-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_sequence_format: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIndexBody = String;

pub type TextIndexEntryBibliography = TextIndexEntryBibliographyAttrs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextIndexEntryBibliographyAttrs {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:bibliography-data-field")]
    pub bibliography_data_field: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIndexEntryChapter = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextIndexEntryChapterAttrs {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "@text:outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_level: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIndexEntryLinkEnd = String;

pub type TextIndexEntryLinkStart = String;

pub type TextIndexEntryPageNumber = String;

pub type TextIndexEntrySpan = String;

pub type TextIndexEntryTabStop = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextIndexEntryTabStopAttrs {
    #[serde(rename = "@style:leader-char")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_char: Option<Character>,
    #[serde(rename = "@style:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@style:position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Length>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIndexEntryText = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextIndexName {
    #[serde(rename = "@text:index-name")]
    pub index_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextIndexScopeAttr {
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextIndexSourceStyle = String;

pub type TextIndexSourceStyles = String;

pub type TextIndexTitle = String;

pub type TextIndexTitleTemplate = String;

pub type TextLinenumberingConfiguration = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextLinenumberingConfigurationAttlist {
    #[serde(rename = "@text:number-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_lines: Option<Boolean>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:increment")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub increment: Option<NonNegativeInteger>,
    #[serde(rename = "@text:number-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_position: Option<String>,
    #[serde(rename = "@text:offset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<NonNegativeLength>,
    #[serde(rename = "@text:count-empty-lines")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_empty_lines: Option<Boolean>,
    #[serde(rename = "@text:count-in-text-boxes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_in_text_boxes: Option<Boolean>,
    #[serde(rename = "@text:restart-on-page")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_on_page: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextLinenumberingSeparator = String;

pub type TextList = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListAttr {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:continue-numbering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_numbering: Option<Boolean>,
    #[serde(rename = "@text:continue-list")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_list: Option<IDREF>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextListHeader = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListHeaderAttr {
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextListItem = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListItemAttr {
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<NonNegativeInteger>,
    #[serde(rename = "@text:style-override")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_override: Option<StyleNameRef>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListItemContent {
    #[serde(rename = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(rename = "p")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub p: Vec<String>,
    #[serde(rename = "h")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub h: Vec<String>,
    #[serde(rename = "list")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<String>,
    #[serde(rename = "soft-page-break")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub soft_page_break: Vec<()>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextListLevelStyleAttr {
    #[serde(rename = "@text:level")]
    pub level: PositiveInteger,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextListLevelStyleBulletAttr {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:bullet-char")]
    pub bullet_char: Character,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@text:bullet-relative-size")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bullet_relative_size: Option<Percent>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListLevelStyleImageAttr {
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@xlink:actuate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuate: Option<String>,
    #[serde(rename = "binary-data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_data: Option<Base64Binary>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListLevelStyleNumberAttr {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@text:display-levels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_levels: Option<PositiveInteger>,
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextListStyle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextListStyleAttr {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    #[serde(rename = "@style:display-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "@text:consecutive-numbering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consecutive_numbering: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextListStyleContent {
    #[serde(rename = "list-level-style-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_style_number: Option<String>,
    #[serde(rename = "list-level-style-bullet")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_style_bullet: Option<String>,
    #[serde(rename = "list-level-style-image")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_level_style_image: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextMetaAttlist {
    #[serde(rename = "@xhtml:about")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<URIorSafeCURIE>,
    #[serde(rename = "@xhtml:property")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<CURIEs>,
    #[serde(rename = "@xhtml:datatype")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<CURIE>,
    #[serde(rename = "@xhtml:content")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMetaFieldAttlist {
    #[serde(rename = "@xml:id")]
    pub id: ID,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNoteClass {
    #[serde(rename = "@text:note-class")]
    pub note_class: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNoteRefContent {
    #[serde(rename = "@text:reference-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_format: Option<CommonRefFormatValues>,
    #[serde(rename = "@text:note-class")]
    pub note_class: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextNotesConfiguration = TextNotesConfigurationContent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNotesConfigurationContent {
    #[serde(rename = "@text:note-class")]
    pub note_class: String,
    #[serde(rename = "@text:citation-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:citation-body-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_body_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:default-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:master-page-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_page_name: Option<StyleNameRef>,
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<NonNegativeInteger>,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@text:start-numbering-at")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_numbering_at: Option<String>,
    #[serde(rename = "@text:footnotes-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnotes_position: Option<String>,
    #[serde(rename = "note-continuation-notice-forward")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_continuation_notice_forward: Option<String>,
    #[serde(rename = "note-continuation-notice-backward")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_continuation_notice_backward: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextNumber = String;

pub type TextNumberedParagraph = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNumberedParagraphAttr {
    #[serde(rename = "@text:list-id")]
    pub list_id: NCName,
    #[serde(rename = "@text:level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<PositiveInteger>,
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:continue-numbering")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_numbering: Option<Boolean>,
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<NonNegativeInteger>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextObjectIndex = String;

pub type TextObjectIndexEntryTemplate = TextIllustrationIndexEntryContent;

pub type TextObjectIndexSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextObjectIndexSourceAttrs {
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    #[serde(rename = "@text:use-spreadsheet-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_spreadsheet_objects: Option<Boolean>,
    #[serde(rename = "@text:use-math-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_math_objects: Option<Boolean>,
    #[serde(rename = "@text:use-draw-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_draw_objects: Option<Boolean>,
    #[serde(rename = "@text:use-chart-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_chart_objects: Option<Boolean>,
    #[serde(rename = "@text:use-other-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_other_objects: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextOutlineLevel {
    #[serde(rename = "@text:outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_level: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextOutlineLevelStyle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOutlineLevelStyleAttlist {
    #[serde(rename = "@text:level")]
    pub level: PositiveInteger,
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@style:num-prefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_prefix: Option<String>,
    #[serde(rename = "@style:num-suffix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_suffix: Option<String>,
    #[serde(rename = "@text:display-levels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_levels: Option<PositiveInteger>,
    #[serde(rename = "@text:start-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextOutlineStyle = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOutlineStyleAttr {
    #[serde(rename = "@style:name")]
    pub name: StyleName,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextP = String;

pub type TextPage = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPageAttlist {
    #[serde(rename = "@text:master-page-name")]
    pub master_page_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPageContinuationAttlist {
    #[serde(rename = "@text:select-page")]
    pub select_page: String,
    #[serde(rename = "@text:string-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextPageNumberAttlist {
    #[serde(rename = "@style:num-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(rename = "@style:num-letter-sync")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_letter_sync: Option<Boolean>,
    #[serde(rename = "@text:fixed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Boolean>,
    #[serde(rename = "@text:page-adjust")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_adjust: Option<Integer>,
    #[serde(rename = "@text:select-page")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select_page: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextPageSequence = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPlaceholderAttlist {
    #[serde(rename = "@text:placeholder-type")]
    pub placeholder_type: String,
    #[serde(rename = "@text:description")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextRelativeTabStopPositionAttr {
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextSection = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSectionAttlist {
    #[serde(rename = "@text:style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:name")]
    pub name: String,
    #[serde(rename = "@text:protected")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<Boolean>,
    #[serde(rename = "@text:protection-key")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key: Option<String>,
    #[serde(rename = "@text:protection-key-digest-algorithm")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_key_digest_algorithm: Option<AnyIRI>,
    #[serde(rename = "@xml:id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "@text:condition")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextSectionSource = TextSectionSourceAttr;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextSectionSourceAttr {
    #[serde(rename = "@xlink:type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "@xlink:href")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<AnyIRI>,
    #[serde(rename = "@xlink:show")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(rename = "@text:section-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_name: Option<String>,
    #[serde(rename = "@text:filter-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSectionSourceDde {
    #[serde(rename = "dde-source")]
    pub dde_source: String,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextSequenceDecl = TextSequenceDeclAttlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSequenceDeclAttlist {
    #[serde(rename = "@text:name")]
    pub name: VariableName,
    #[serde(rename = "@text:display-outline-level")]
    pub display_outline_level: NonNegativeInteger,
    #[serde(rename = "@text:separation-character")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separation_character: Option<Character>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextSequenceRefContent {
    #[serde(rename = "@text:reference-format")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_format: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextSequenceRefName {
    #[serde(rename = "@text:ref-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_name: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextSetPageVariableAttlist {
    #[serde(rename = "@text:active")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<Boolean>,
    #[serde(rename = "@text:page-adjust")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_adjust: Option<Integer>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextSoftPageBreak = ();

pub type TextSortKey = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSortKeyAttlist {
    #[serde(rename = "@text:key")]
    pub key: String,
    #[serde(rename = "@text:sort-ascending")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_ascending: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextStyleName {
    #[serde(rename = "@form:text-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_style_name: Option<StyleNameRef>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTabAttr {
    #[serde(rename = "@text:tab-ref")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_ref: Option<NonNegativeInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextTableIndex = String;

pub type TextTableIndexEntryTemplate = TextIllustrationIndexEntryContent;

pub type TextTableIndexSource = String;

pub type TextTableOfContent = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTableOfContentChildren {
    #[serde(rename = "index-entry-chapter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_chapter: Option<String>,
    #[serde(rename = "index-entry-page-number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_page_number: Option<String>,
    #[serde(rename = "index-entry-text")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_text: Option<String>,
    #[serde(rename = "index-entry-span")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_span: Option<String>,
    #[serde(rename = "index-entry-tab-stop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_tab_stop: Option<String>,
    #[serde(rename = "index-entry-link-start")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_link_start: Option<String>,
    #[serde(rename = "index-entry-link-end")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_entry_link_end: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

pub type TextTableOfContentEntryTemplate = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextTableOfContentEntryTemplateAttlist {
    #[serde(rename = "@text:outline-level")]
    pub outline_level: PositiveInteger,
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextTableOfContentSource = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTableOfContentSourceAttlist {
    #[serde(rename = "@text:outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_level: Option<PositiveInteger>,
    #[serde(rename = "@text:use-outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_outline_level: Option<Boolean>,
    #[serde(rename = "@text:use-index-marks")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_index_marks: Option<Boolean>,
    #[serde(rename = "@text:use-index-source-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_index_source_styles: Option<Boolean>,
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTemplateNameAttlist {
    #[serde(rename = "@text:display")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTimeAttlist {
    #[serde(rename = "@text:fixed")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Boolean>,
    #[serde(rename = "@style:data-style-name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_style_name: Option<StyleNameRef>,
    #[serde(rename = "@text:time-value")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_value: Option<TimeOrDateTime>,
    #[serde(rename = "@text:time-adjust")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_adjust: Option<Duration>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextTocMarkStartAttrs {
    #[serde(rename = "@text:id")]
    pub id: String,
    #[serde(rename = "@text:outline-level")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_level: Option<PositiveInteger>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTrackedChanges {
    #[serde(rename = "tracked-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_changes: Option<String>,
    /// Unknown child elements captured for roundtrip fidelity.
    #[cfg(feature = "extra-children")]
    #[serde(skip)]
    #[cfg(feature = "extra-children")]
    pub extra_children: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextTrackedChangesAttr {
    #[serde(rename = "@text:track-changes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_changes: Option<Boolean>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextUserFieldDecl = String;

pub type TextUserIndex = String;

pub type TextUserIndexEntryTemplate = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextUserIndexEntryTemplateAttrs {
    #[serde(rename = "@text:outline-level")]
    pub outline_level: PositiveInteger,
    #[serde(rename = "@text:style-name")]
    pub style_name: StyleNameRef,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextUserIndexSource = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextUserIndexSourceAttr {
    #[serde(rename = "@text:index-scope")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_scope: Option<String>,
    #[serde(rename = "@text:relative-tab-stop-position")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_tab_stop_position: Option<Boolean>,
    #[serde(rename = "@text:use-index-marks")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_index_marks: Option<Boolean>,
    #[serde(rename = "@text:use-index-source-styles")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_index_source_styles: Option<Boolean>,
    #[serde(rename = "@text:use-graphics")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_graphics: Option<Boolean>,
    #[serde(rename = "@text:use-tables")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_tables: Option<Boolean>,
    #[serde(rename = "@text:use-floating-frames")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_floating_frames: Option<Boolean>,
    #[serde(rename = "@text:use-objects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_objects: Option<Boolean>,
    #[serde(rename = "@text:copy-outline-levels")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_outline_levels: Option<Boolean>,
    #[serde(rename = "@text:index-name")]
    pub index_name: String,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type TextVariableDecl = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeOrDateTime;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XformsBindAttlist {
    #[serde(rename = "@xforms:bind")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

pub type XformsModel = AnyAttListOrElements;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlId {
    #[serde(rename = "@xml:id")]
    pub id: ID,
    /// Unknown attributes captured for roundtrip fidelity.
    #[cfg(feature = "extra-attrs")]
    #[serde(skip)]
    #[cfg(feature = "extra-attrs")]
    #[serde(default)]
    #[cfg(feature = "extra-attrs")]
    pub extra_attrs: std::collections::HashMap<String, String>,
}

