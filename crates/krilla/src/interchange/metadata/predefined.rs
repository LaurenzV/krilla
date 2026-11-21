use std::sync::LazyLock;

use xmp_writer::Namespace as XmpNamespace;

use super::xmp::{
    Namespace, PredefinedStruct, PropertyDescription, Schema, SimpleType, StructType, ValueType,
};

/// Schemas describing the properties krilla writes itself, either from the
/// [`Metadata`] or based on the configuration.
pub(super) fn managed_schemas() -> &'static [Schema] {
    static SCHEMAS: LazyLock<Vec<Schema>> = LazyLock::new(|| {
        use SimpleType::*;
        let schema = |ns: XmpNamespace, properties: &[(&str, ValueType, &str)]| {
            properties.iter().fold(
                Schema::new(Namespace::new(ns.prefix(), ns.url())).name(ns.name()),
                |schema, (name, value_type, description)| {
                    schema.property(PropertyDescription::new(
                        *name,
                        value_type.clone(),
                        *description,
                    ))
                },
            )
        };

        vec![
            schema(
                XmpNamespace::DublinCore,
                &[
                    (
                        "title",
                        ValueType::LanguageAlternative,
                        "The title of the document",
                    ),
                    (
                        "description",
                        ValueType::LanguageAlternative,
                        "A textual description of the content of the document",
                    ),
                    (
                        "creator",
                        ValueType::ordered_array(ProperName),
                        "The authors of the document",
                    ),
                    (
                        "language",
                        ValueType::unordered_array(Locale),
                        "The languages used in the document",
                    ),
                    ("format", MimeType.into(), "The file format of the document"),
                ],
            ),
            schema(
                XmpNamespace::Xmp,
                &[
                    (
                        "CreatorTool",
                        AgentName.into(),
                        "The name of the first known tool used to create the document",
                    ),
                    (
                        "CreateDate",
                        Date.into(),
                        "The date the document was created",
                    ),
                    (
                        "ModifyDate",
                        Date.into(),
                        "The date the document was last modified",
                    ),
                ],
            ),
            schema(
                XmpNamespace::XmpMedia,
                &[
                    (
                        "InstanceID",
                        Uri.into(),
                        "An identifier for this incarnation of the document",
                    ),
                    (
                        "DocumentID",
                        Uri.into(),
                        "The common identifier for all versions of the document",
                    ),
                    (
                        "VersionID",
                        Text.into(),
                        "The version identifier of the document",
                    ),
                    (
                        "RenditionClass",
                        RenditionClass.into(),
                        "The rendition class of the document",
                    ),
                    (
                        "History",
                        ValueType::ordered_array(PredefinedStruct::ResourceEvent),
                        "The high-level actions that resulted in this document",
                    ),
                ],
            ),
            schema(
                XmpNamespace::XmpPaged,
                &[(
                    "NPages",
                    Integer.into(),
                    "The number of pages in the document",
                )],
            ),
            schema(
                XmpNamespace::AdobePdf,
                &[
                    (
                        "Keywords",
                        Text.into(),
                        "Keywords associated with the document",
                    ),
                    ("PDFVersion", Text.into(), "The PDF version of the document"),
                    (
                        "Producer",
                        AgentName.into(),
                        "The name of the tool that created the PDF",
                    ),
                ],
            ),
            schema(
                XmpNamespace::PdfAId,
                &[
                    (
                        "part",
                        Integer.into(),
                        "The part of ISO 19005 the document conforms to",
                    ),
                    (
                        "rev",
                        Integer.into(),
                        "The year of the revision of the part",
                    ),
                    (
                        "conformance",
                        Text.into(),
                        "The conformance level of the part",
                    ),
                    ("amd", Text.into(), "The amendment identifier"),
                    ("corr", Text.into(), "The corrigendum identifier"),
                ],
            ),
            schema(
                XmpNamespace::PdfUAId,
                &[
                    (
                        "part",
                        Integer.into(),
                        "The part of ISO 14289 the document conforms to",
                    ),
                    (
                        "rev",
                        Integer.into(),
                        "The year of the revision of the part",
                    ),
                    ("amd", Text.into(), "The amendment identifier"),
                    ("corr", Text.into(), "The corrigendum identifier"),
                ],
            ),
        ]
    });

    &SCHEMAS
}

/// The prefix of the managed property `name` in `uri`, if it is one.
pub(super) fn managed_prefix(uri: &str, name: &str) -> Option<&'static str> {
    const PDFA_EXTENSION_NAMESPACES: [XmpNamespace; 5] = [
        XmpNamespace::PdfAExtension,
        XmpNamespace::PdfASchema,
        XmpNamespace::PdfAProperty,
        XmpNamespace::PdfAType,
        XmpNamespace::PdfAField,
    ];

    managed_schemas()
        .iter()
        .find(|schema| schema.namespace.uri() == uri && schema.get(name).is_some())
        .map(|schema| schema.namespace.prefix())
        .or_else(|| {
            PDFA_EXTENSION_NAMESPACES
                .iter()
                .find(|ns| ns.url() == uri)
                .map(|ns| ns.prefix())
        })
}

/// The properties PDF/A-1 predefines: those of the schemas in the XMP
/// Specification, January 2004, plus the PDF/A identification schema of
/// ISO 19005-1.
///
/// Sorted by namespace URI and property name.
static PDF_A1_PREDEFINED_PROPERTIES: &[(&str, &[&str])] = &[
    (
        "http://ns.adobe.com/exif/1.0/",
        &[
            "ApertureValue",
            "BrightnessValue",
            "CFAPattern",
            "ColorSpace",
            "ComponentsConfiguration",
            "CompressedBitsPerPixel",
            "Contrast",
            "CustomRendered",
            "DateTimeDigitized",
            "DateTimeOriginal",
            "DeviceSettingDescription",
            "DigitalZoomRatio",
            "ExifVersion",
            "ExposureBiasValue",
            "ExposureIndex",
            "ExposureMode",
            "ExposureProgram",
            "ExposureTime",
            "FNumber",
            "FileSource",
            "Flash",
            "FlashEnergy",
            "FlashpixVersion",
            "FocalLength",
            "FocalLengthIn35mmFilm",
            "FocalPlaneResolutionUnit",
            "FocalPlaneXResolution",
            "FocalPlaneYResolution",
            "GPSAltitude",
            "GPSAltitudeRef",
            "GPSAreaInformation",
            "GPSDOP",
            "GPSDestBearing",
            "GPSDestBearingRef",
            "GPSDestDistance",
            "GPSDestDistanceRef",
            "GPSDestLatitude",
            "GPSDestLongitude",
            "GPSDifferential",
            "GPSImgDirection",
            "GPSImgDirectionRef",
            "GPSLatitude",
            "GPSLongitude",
            "GPSMapDatum",
            "GPSMeasureMode",
            "GPSProcessingMethod",
            "GPSSatellites",
            "GPSSpeed",
            "GPSSpeedRef",
            "GPSStatus",
            "GPSTimeStamp",
            "GPSTrack",
            "GPSTrackRef",
            "GPSVersionID",
            "GainControl",
            "ISOSpeedRatings",
            "ImageUniqueID",
            "LightSource",
            "MakerNote",
            "MaxApertureValue",
            "MeteringMode",
            "OECF",
            "PixelXDimension",
            "PixelYDimension",
            "RelatedSoundFile",
            "Saturation",
            "SceneCaptureType",
            "SceneType",
            "SensingMethod",
            "Sharpness",
            "ShutterSpeedValue",
            "SpatialFrequencyResponse",
            "SpectralSensitivity",
            "SubjectArea",
            "SubjectDistance",
            "SubjectDistanceRange",
            "SubjectLocation",
            "UserComment",
            "WhiteBalance",
        ],
    ),
    (
        "http://ns.adobe.com/pdf/1.3/",
        &["Keywords", "PDFVersion", "Producer"],
    ),
    (
        "http://ns.adobe.com/photoshop/1.0/",
        &[
            "AuthorsPosition",
            "CaptionWriter",
            "Category",
            "City",
            "Country",
            "Credit",
            "DateCreated",
            "Headline",
            "Instructions",
            "Source",
            "State",
            "SupplementalCategories",
            "TransmissionReference",
            "Urgency",
        ],
    ),
    (
        "http://ns.adobe.com/tiff/1.0/",
        &[
            "Artist",
            "BitsPerSample",
            "Compression",
            "Copyright",
            "DateTime",
            "ImageDescription",
            "ImageLength",
            "ImageWidth",
            "Make",
            "Model",
            "Orientation",
            "PhotometricInterpretation",
            "PlanarConfiguration",
            "PrimaryChromaticities",
            "ReferenceBlackWhite",
            "ResolutionUnit",
            "SamplesPerPixel",
            "Software",
            "TransferFunction",
            "WhitePoint",
            "XResolution",
            "YCbCrCoefficients",
            "YCbCrPositioning",
            "YCbCrSubSampling",
            "YResolution",
        ],
    ),
    (
        "http://ns.adobe.com/xap/1.0/",
        &[
            "Advisory",
            "BaseURL",
            "CreateDate",
            "CreatorTool",
            "Identifier",
            "MetadataDate",
            "ModifyDate",
            "Nickname",
            "Thumbnails",
        ],
    ),
    ("http://ns.adobe.com/xap/1.0/bj/", &["JobRef"]),
    (
        "http://ns.adobe.com/xap/1.0/mm/",
        &[
            "DerivedFrom",
            "DocumentID",
            "History",
            "LastURL",
            "ManageTo",
            "ManageUI",
            "ManagedFrom",
            "Manager",
            "ManagerVariant",
            "RenditionClass",
            "RenditionOf",
            "RenditionParams",
            "SaveID",
            "VersionID",
            "Versions",
        ],
    ),
    (
        "http://ns.adobe.com/xap/1.0/rights/",
        &[
            "Certificate",
            "Marked",
            "Owner",
            "UsageTerms",
            "WebStatement",
        ],
    ),
    (
        "http://ns.adobe.com/xap/1.0/t/pg/",
        &["MaxPageSize", "NPages"],
    ),
    (
        "http://purl.org/dc/elements/1.1/",
        &[
            "contributor",
            "coverage",
            "creator",
            "date",
            "description",
            "format",
            "identifier",
            "language",
            "publisher",
            "relation",
            "rights",
            "source",
            "subject",
            "title",
            "type",
        ],
    ),
    (
        "http://www.aiim.org/pdfa/ns/id/",
        &["amd", "conformance", "part"],
    ),
];

/// The properties PDF/A-2 and PDF/A-3 predefine in addition to
/// [`PDF_A1_PREDEFINED_PROPERTIES`]: those the XMP Specification, September
/// 2005, added to the January 2004 edition, plus `pdfaid:corr` of ISO 19005-2.
///
/// Sorted by namespace URI and property name.
static PDF_A2_A3_ADDITIONAL_PROPERTIES: &[(&str, &[&str])] = &[
    (
        "http://ns.adobe.com/camera-raw-settings/1.0/",
        &[
            "AutoBrightness",
            "AutoContrast",
            "AutoExposure",
            "AutoShadows",
            "BlueHue",
            "BlueSaturation",
            "Brightness",
            "CameraProfile",
            "ChromaticAberrationB",
            "ChromaticAberrationR",
            "ColorNoiseReduction",
            "Contrast",
            "CropAngle",
            "CropBottom",
            "CropHeight",
            "CropLeft",
            "CropRight",
            "CropTop",
            "CropUnits",
            "CropWidth",
            "Exposure",
            "GreenHue",
            "GreenSaturation",
            "HasCrop",
            "HasSettings",
            "LuminanceSmoothing",
            "RawFileName",
            "RedHue",
            "RedSaturation",
            "Saturation",
            "ShadowTint",
            "Shadows",
            "Sharpness",
            "Temperature",
            "Tint",
            "ToneCurve",
            "ToneCurveName",
            "Version",
            "VignetteAmount",
            "VignetteMidpoint",
            "WhiteBalance",
        ],
    ),
    (
        "http://ns.adobe.com/exif/1.0/aux/",
        &["Lens", "SerialNumber"],
    ),
    ("http://ns.adobe.com/xap/1.0/", &["Label", "Rating"]),
    ("http://ns.adobe.com/xap/1.0/mm/", &["InstanceID"]),
    (
        "http://ns.adobe.com/xap/1.0/t/pg/",
        &["Colorants", "Fonts", "PlateNames"],
    ),
    (
        "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        &[
            "absPeakAudioFilePath",
            "album",
            "altTapeName",
            "altTimecode",
            "artist",
            "audioChannelType",
            "audioCompressor",
            "audioModDate",
            "audioSampleRate",
            "audioSampleType",
            "beatSpliceParams",
            "composer",
            "contributedMedia",
            "copyright",
            "duration",
            "engineer",
            "fileDataRate",
            "genre",
            "instrument",
            "introTime",
            "key",
            "logComment",
            "loop",
            "markers",
            "metadataModDate",
            "numberOfBeats",
            "outCue",
            "projectRef",
            "pullDown",
            "relativePeakAudioFilePath",
            "relativeTimestamp",
            "releaseDate",
            "resampleParams",
            "scaleType",
            "scene",
            "shotDate",
            "shotLocation",
            "shotName",
            "speakerPlacement",
            "startTimecode",
            "stretchMode",
            "tapeName",
            "tempo",
            "timeScaleParams",
            "timeSignature",
            "trackNumber",
            "videoAlphaMode",
            "videoAlphaPremultipleColor",
            "videoAlphaUnityIsTransparent",
            "videoColorSpace",
            "videoCompressor",
            "videoFieldOrder",
            "videoFrameRate",
            "videoFrameSize",
            "videoModDate",
            "videoPixelAspectRatio",
            "videoPixelDepth",
        ],
    ),
    ("http://www.aiim.org/pdfa/ns/id/", &["corr"]),
];

/// The properties of [`PDF_A1_PREDEFINED_PROPERTIES`] that the XMP
/// Specification, September 2005, no longer contains.
///
/// Sorted by namespace URI and property name.
static PDF_A1_ONLY_PROPERTIES: &[(&str, &[&str])] =
    &[("http://ns.adobe.com/exif/1.0/", &["MakerNote"])];

/// Whether PDF/A exempts the property `name` in `uri` from an extension
/// schema description. `xmp_2005` selects the PDF/A-2/-3 rules over the
/// PDF/A-1 rules; see [`Validators::uses_xmp_2005_predefined_schemas`].
pub(super) fn is_predefined_pdfa_property(uri: &str, name: &str, xmp_2005: bool) -> bool {
    let pdf_a1 = contains_property(PDF_A1_PREDEFINED_PROPERTIES, uri, name);
    if xmp_2005 {
        (pdf_a1 && !contains_property(PDF_A1_ONLY_PROPERTIES, uri, name))
            || contains_property(PDF_A2_A3_ADDITIONAL_PROPERTIES, uri, name)
    } else {
        pdf_a1
    }
}

fn contains_property(properties: &[(&str, &[&str])], uri: &str, name: &str) -> bool {
    properties
        .binary_search_by_key(&uri, |(uri, _)| uri)
        .is_ok_and(|i| properties[i].1.binary_search(&name).is_ok())
}

/// The fields of the predefined structure types, as listed in TechNote 0008,
/// chapter 3. Returns `None` for types whose fields aren't known.
pub(super) fn predefined_struct_type(ty: PredefinedStruct) -> Option<&'static StructType> {
    use std::sync::LazyLock;

    fn struct_type(
        prefix: &str,
        uri: &str,
        name: &str,
        fields: &[(&str, ValueType)],
    ) -> StructType {
        fields.iter().fold(
            StructType::new(Namespace::new(prefix, uri), name, ""),
            |ty, (name, value_type)| {
                ty.field(PropertyDescription::new(*name, value_type.clone(), ""))
            },
        )
    }

    static TYPES: LazyLock<[StructType; 6]> = LazyLock::new(|| {
        use SimpleType::*;
        let open = |ty| ValueType::OpenChoice(ty, vec![]);
        [
            struct_type(
                "stDim",
                "http://ns.adobe.com/xap/1.0/sType/Dimensions#",
                "Dimensions",
                &[("w", Real.into()), ("h", Real.into()), ("unit", open(Text))],
            ),
            struct_type(
                "xmpGImg",
                "http://ns.adobe.com/xap/1.0/g/img/",
                "Thumbnail",
                &[
                    ("height", Integer.into()),
                    ("width", Integer.into()),
                    ("format", open(Text)),
                    ("image", Text.into()),
                ],
            ),
            struct_type(
                "stEvt",
                "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
                "ResourceEvent",
                &[
                    ("action", open(Text)),
                    ("instanceID", Uri.into()),
                    ("parameters", Text.into()),
                    ("softwareAgent", AgentName.into()),
                    ("when", Date.into()),
                ],
            ),
            struct_type(
                "stRef",
                "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
                "ResourceRef",
                &[
                    ("instanceID", Uri.into()),
                    ("documentID", Uri.into()),
                    ("versionID", Text.into()),
                    ("renditionClass", RenditionClass.into()),
                    ("renditionParams", Text.into()),
                    ("manager", AgentName.into()),
                    ("managerVariant", Text.into()),
                    ("manageTo", Uri.into()),
                    ("manageUI", Uri.into()),
                ],
            ),
            struct_type(
                "stVer",
                "http://ns.adobe.com/xap/1.0/sType/Version#",
                "Version",
                &[
                    ("comments", Text.into()),
                    ("event", PredefinedStruct::ResourceEvent.into()),
                    ("modifyDate", Date.into()),
                    ("modifier", ProperName.into()),
                    ("version", Text.into()),
                ],
            ),
            struct_type(
                "stJob",
                "http://ns.adobe.com/xap/1.0/sType/Job#",
                "Job",
                &[
                    ("name", Text.into()),
                    ("id", Text.into()),
                    ("url", Url.into()),
                ],
            ),
        ]
    });

    let index = match ty {
        PredefinedStruct::Dimensions => 0,
        PredefinedStruct::Thumbnail => 1,
        PredefinedStruct::ResourceEvent => 2,
        PredefinedStruct::ResourceRef => 3,
        PredefinedStruct::Version => 4,
        PredefinedStruct::Job => 5,
        PredefinedStruct::Colorant | PredefinedStruct::Font => return None,
    };

    Some(&TYPES[index])
}

#[cfg(test)]
mod tests {
    use super::{
        PDF_A1_ONLY_PROPERTIES, PDF_A1_PREDEFINED_PROPERTIES, PDF_A2_A3_ADDITIONAL_PROPERTIES,
    };

    #[test]
    fn predefined_property_lists_are_sorted() {
        for list in [
            PDF_A1_PREDEFINED_PROPERTIES,
            PDF_A2_A3_ADDITIONAL_PROPERTIES,
            PDF_A1_ONLY_PROPERTIES,
        ] {
            assert!(list.is_sorted_by_key(|(uri, _)| uri));
            assert!(list.iter().all(|(_, names)| names.is_sorted()));
        }
    }
}
