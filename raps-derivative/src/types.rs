// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Type definitions for the Model Derivative API.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// APS data center regions for Model Derivative service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MdRegion {
    #[default]
    US,
    EMEA,
    AUS,
    CAN,
    DEU,
    IND,
    JPN,
    GBR,
}

impl std::fmt::Display for MdRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MdRegion::US => "US",
            MdRegion::EMEA => "EMEA",
            MdRegion::AUS => "AUS",
            MdRegion::CAN => "CAN",
            MdRegion::DEU => "DEU",
            MdRegion::IND => "IND",
            MdRegion::JPN => "JPN",
            MdRegion::GBR => "GBR",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for MdRegion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_uppercase().as_str() {
            "US" => Ok(MdRegion::US),
            "EMEA" => Ok(MdRegion::EMEA),
            "AUS" => Ok(MdRegion::AUS),
            "CAN" => Ok(MdRegion::CAN),
            "DEU" => Ok(MdRegion::DEU),
            "IND" => Ok(MdRegion::IND),
            "JPN" => Ok(MdRegion::JPN),
            "GBR" => Ok(MdRegion::GBR),
            _ => anyhow::bail!(
                "Invalid region '{}'. Valid values: US, EMEA, AUS, CAN, DEU, IND, JPN, GBR",
                s
            ),
        }
    }
}

/// Supported output formats for translation
#[derive(Debug, Clone, Copy, Serialize)]
pub enum OutputFormat {
    /// Streaming format for Viewer (recommended)
    #[serde(rename = "svf2")]
    Svf2,
    /// Legacy streaming format
    #[serde(rename = "svf")]
    Svf,
    /// Thumbnail images
    #[serde(rename = "thumbnail")]
    Thumbnail,
    /// OBJ format (mesh export)
    #[serde(rename = "obj")]
    Obj,
    /// STL format (3D printing)
    #[serde(rename = "stl")]
    Stl,
    /// STEP format (CAD interchange)
    #[serde(rename = "step")]
    Step,
    /// IGES format (CAD interchange)
    #[serde(rename = "iges")]
    Iges,
    /// IFC format (BIM)
    #[serde(rename = "ifc")]
    Ifc,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Svf2 => write!(f, "SVF2 (Viewer)"),
            OutputFormat::Svf => write!(f, "SVF (Legacy Viewer)"),
            OutputFormat::Thumbnail => write!(f, "Thumbnail"),
            OutputFormat::Obj => write!(f, "OBJ (Mesh)"),
            OutputFormat::Stl => write!(f, "STL (3D Print)"),
            OutputFormat::Step => write!(f, "STEP (CAD)"),
            OutputFormat::Iges => write!(f, "IGES (CAD)"),
            OutputFormat::Ifc => write!(f, "IFC (BIM)"),
        }
    }
}

impl OutputFormat {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Svf2,
            Self::Svf,
            Self::Thumbnail,
            Self::Obj,
            Self::Stl,
            Self::Step,
            Self::Iges,
            Self::Ifc,
        ]
    }

    pub fn type_name(&self) -> &str {
        match self {
            OutputFormat::Svf2 => "svf2",
            OutputFormat::Svf => "svf",
            OutputFormat::Thumbnail => "thumbnail",
            OutputFormat::Obj => "obj",
            OutputFormat::Stl => "stl",
            OutputFormat::Step => "step",
            OutputFormat::Iges => "iges",
            OutputFormat::Ifc => "ifc",
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "svf2" => Ok(Self::Svf2),
            "svf" => Ok(Self::Svf),
            "thumbnail" => Ok(Self::Thumbnail),
            "obj" => Ok(Self::Obj),
            "stl" => Ok(Self::Stl),
            "step" => Ok(Self::Step),
            "iges" => Ok(Self::Iges),
            "ifc" => Ok(Self::Ifc),
            _ => Err(format!(
                "Invalid output format: {}. Use: {}",
                s,
                Self::all()
                    .iter()
                    .map(OutputFormat::type_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

/// Request to start a translation job
#[derive(Debug, Serialize)]
pub struct TranslationRequest {
    pub input: TranslationInput,
    pub output: TranslationOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationInput {
    pub urn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_urn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_filename: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TranslationOutput {
    pub destination: OutputDestination,
    pub formats: Vec<OutputFormatSpec>,
}

#[derive(Debug, Serialize)]
pub struct OutputDestination {
    pub region: String,
}

#[derive(Debug, Serialize)]
pub struct OutputFormatSpec {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<Vec<String>>,
}

/// Translation job response
#[derive(Debug, Deserialize)]
pub struct TranslationResponse {
    pub result: String,
    pub urn: String,
    #[serde(rename = "acceptedJobs")]
    pub accepted_jobs: Option<AcceptedJobs>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptedJobs {
    pub output: OutputJobInfo,
}

#[derive(Debug, Deserialize)]
pub struct OutputJobInfo {
    pub formats: Vec<FormatJobInfo>,
}

#[derive(Debug, Deserialize)]
pub struct FormatJobInfo {
    #[serde(rename = "type")]
    pub format_type: String,
}

/// Manifest response (translation status and derivatives)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "type")]
    pub manifest_type: String,
    pub has_thumbnail: String,
    pub status: String,
    pub progress: String,
    pub region: String,
    pub urn: String,
    pub version: Option<String>,
    #[serde(default)]
    pub derivatives: Vec<Derivative>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Derivative {
    pub name: Option<String>,
    pub has_thumbnail: Option<String>,
    pub status: String,
    pub progress: Option<String>,
    pub output_type: String,
    #[serde(default)]
    pub children: Vec<DerivativeChild>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivativeChild {
    pub guid: String,
    #[serde(rename = "type")]
    pub child_type: String,
    pub role: String,
    pub name: Option<String>,
    pub status: Option<String>,
    /// URN for downloadable derivatives
    pub urn: Option<String>,
    /// MIME type for downloadable files
    pub mime: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    #[serde(default)]
    pub children: Vec<DerivativeChild>,
}

/// Information about a downloadable derivative
#[derive(Debug, Clone, Serialize)]
pub struct DownloadableDerivative {
    pub guid: String,
    pub name: String,
    pub output_type: String,
    pub role: String,
    pub urn: String,
    pub mime: Option<String>,
    pub size: Option<u64>,
}

// ============== METADATA TYPES ==============

/// Response from GET /metadata -- list model views/viewables
#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataResponse {
    pub data: MetadataData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub metadata: Vec<ModelView>,
}

/// A single view/viewable within a translated model
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub guid: String,
    pub name: String,
    pub role: String,
    #[serde(rename = "mime")]
    pub mime_type: Option<String>,
    pub has_thumbnail: Option<String>,
    pub progress: Option<String>,
}

/// Response from GET /metadata/{guid} -- object tree hierarchy
#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectTreeResponse {
    pub data: ObjectTreeData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectTreeData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub objects: Vec<ObjectTreeNode>,
}

/// A node in the model's object tree
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObjectTreeNode {
    #[serde(rename = "objectid")]
    pub object_id: i64,
    pub name: String,
    #[serde(default)]
    pub objects: Vec<ObjectTreeNode>,
}

/// Response from GET/POST /metadata/{guid}/properties
#[derive(Debug, Deserialize, Serialize)]
pub struct PropertiesResponse {
    pub data: PropertiesData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PropertiesData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub collection: Vec<PropertyObject>,
}

/// A single object's properties
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyObject {
    #[serde(rename = "objectid")]
    pub object_id: i64,
    pub name: String,
    pub external_id: Option<String>,
    #[serde(default)]
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Request body for POST /metadata/{guid}/properties:query
#[derive(Debug, Serialize)]
pub struct PropertyQuery {
    pub query: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PropertyPagination>,
}

impl PropertyQuery {
    /// Create a query filtering by object IDs
    pub fn by_object_ids(ids: Vec<i64>) -> Self {
        let mut filter: Vec<serde_json::Value> =
            vec![serde_json::Value::String("objectid".to_string())];
        filter.extend(
            ids.into_iter()
                .map(|id| serde_json::Value::Number(serde_json::Number::from(id))),
        );
        Self {
            query: serde_json::json!({ "$in": filter }),
            fields: None,
            pagination: None,
        }
    }
}

/// Pagination for property queries
#[derive(Debug, Serialize)]
pub struct PropertyPagination {
    pub offset: usize,
    pub limit: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_serialization() {
        assert_eq!(
            serde_json::to_string(&OutputFormat::Svf2).unwrap(),
            "\"svf2\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Obj).unwrap(),
            "\"obj\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Ifc).unwrap(),
            "\"ifc\""
        );
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Svf2.to_string(), "SVF2 (Viewer)");
        assert_eq!(OutputFormat::Svf.to_string(), "SVF (Legacy Viewer)");
        assert_eq!(OutputFormat::Obj.to_string(), "OBJ (Mesh)");
        assert_eq!(OutputFormat::Stl.to_string(), "STL (3D Print)");
        assert_eq!(OutputFormat::Ifc.to_string(), "IFC (BIM)");
    }

    #[test]
    fn test_output_format_type_name() {
        assert_eq!(OutputFormat::Svf2.type_name(), "svf2");
        assert_eq!(OutputFormat::Obj.type_name(), "obj");
        assert_eq!(OutputFormat::Ifc.type_name(), "ifc");
    }

    #[test]
    fn test_output_format_from_str() {
        assert!(matches!(
            OutputFormat::from_str("svf2"),
            Ok(OutputFormat::Svf2)
        ));
        assert!(matches!(
            OutputFormat::from_str("SVF2"),
            Ok(OutputFormat::Svf2)
        ));
        assert!(matches!(
            OutputFormat::from_str("obj"),
            Ok(OutputFormat::Obj)
        ));
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_output_format_all() {
        let all = OutputFormat::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert!(OutputFormat::from_str("SVF2").is_ok());
        assert!(OutputFormat::from_str("svf2").is_ok());
        assert!(OutputFormat::from_str("Svf2").is_ok());
    }

    #[test]
    fn test_output_format_from_str_all_formats() {
        assert_eq!(OutputFormat::from_str("svf2").unwrap().type_name(), "svf2");
        assert_eq!(OutputFormat::from_str("svf").unwrap().type_name(), "svf");
        assert_eq!(
            OutputFormat::from_str("thumbnail").unwrap().type_name(),
            "thumbnail"
        );
        assert_eq!(OutputFormat::from_str("obj").unwrap().type_name(), "obj");
        assert_eq!(OutputFormat::from_str("stl").unwrap().type_name(), "stl");
        assert_eq!(OutputFormat::from_str("step").unwrap().type_name(), "step");
        assert_eq!(OutputFormat::from_str("iges").unwrap().type_name(), "iges");
        assert_eq!(OutputFormat::from_str("ifc").unwrap().type_name(), "ifc");
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        let result = OutputFormat::from_str("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid output format"));
        assert!(err.contains("svf2")); // Should list valid formats
    }

    #[test]
    fn test_md_region_display() {
        assert_eq!(MdRegion::US.to_string(), "US");
        assert_eq!(MdRegion::EMEA.to_string(), "EMEA");
        assert_eq!(MdRegion::AUS.to_string(), "AUS");
        assert_eq!(MdRegion::CAN.to_string(), "CAN");
        assert_eq!(MdRegion::DEU.to_string(), "DEU");
        assert_eq!(MdRegion::IND.to_string(), "IND");
        assert_eq!(MdRegion::JPN.to_string(), "JPN");
        assert_eq!(MdRegion::GBR.to_string(), "GBR");
    }

    #[test]
    fn test_md_region_from_str() {
        assert_eq!(MdRegion::from_str("emea").unwrap(), MdRegion::EMEA);
        assert_eq!(MdRegion::from_str("US").unwrap(), MdRegion::US);
        assert_eq!(MdRegion::from_str("aus").unwrap(), MdRegion::AUS);
        assert_eq!(MdRegion::from_str("Can").unwrap(), MdRegion::CAN);
        assert_eq!(MdRegion::from_str("deu").unwrap(), MdRegion::DEU);
        assert_eq!(MdRegion::from_str("ind").unwrap(), MdRegion::IND);
        assert_eq!(MdRegion::from_str("jpn").unwrap(), MdRegion::JPN);
        assert_eq!(MdRegion::from_str("gbr").unwrap(), MdRegion::GBR);
        let err = MdRegion::from_str("invalid");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Valid values"));
    }

    #[test]
    fn test_md_region_default_is_us() {
        assert_eq!(MdRegion::default(), MdRegion::US);
    }

    #[test]
    fn test_translation_request_serialization() {
        let request = TranslationRequest {
            input: TranslationInput {
                urn: "test-urn".to_string(),
                compressed_urn: None,
                root_filename: Some("model.rvt".to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: "us".to_string(),
                },
                formats: vec![OutputFormatSpec {
                    format_type: "svf2".to_string(),
                    views: Some(vec!["2d".to_string(), "3d".to_string()]),
                }],
            },
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["input"]["rootFilename"], "model.rvt");
        assert_eq!(json["output"]["destination"]["region"], "us");
    }

    #[test]
    fn test_translation_input_serialization_minimal() {
        let input = TranslationInput {
            urn: "test-urn".to_string(),
            compressed_urn: None,
            root_filename: None,
        };

        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["urn"], "test-urn");
        // Optional fields should not be present
        assert!(json.get("compressedUrn").is_none());
        assert!(json.get("rootFilename").is_none());
    }

    #[test]
    fn test_translation_input_serialization_with_options() {
        let input = TranslationInput {
            urn: "test-urn".to_string(),
            compressed_urn: Some(true),
            root_filename: Some("model.rvt".to_string()),
        };

        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["urn"], "test-urn");
        assert_eq!(json["compressedUrn"], true);
        assert_eq!(json["rootFilename"], "model.rvt");
    }

    #[test]
    fn test_output_format_spec_serialization() {
        let spec = OutputFormatSpec {
            format_type: "svf2".to_string(),
            views: Some(vec!["2d".to_string(), "3d".to_string()]),
        };

        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["type"], "svf2");
        assert_eq!(json["views"], serde_json::json!(["2d", "3d"]));
    }

    #[test]
    fn test_output_format_spec_serialization_no_views() {
        let spec = OutputFormatSpec {
            format_type: "obj".to_string(),
            views: None,
        };

        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["type"], "obj");
        assert!(json.get("views").is_none());
    }

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "type": "manifest",
            "hasThumbnail": "true",
            "status": "success",
            "progress": "complete",
            "region": "US",
            "urn": "test-urn",
            "derivatives": []
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.status, "success");
        assert_eq!(manifest.progress, "complete");
        assert!(manifest.derivatives.is_empty());
    }

    #[test]
    fn test_manifest_with_derivatives() {
        let json = r#"{
            "type": "manifest",
            "hasThumbnail": "true",
            "status": "success",
            "progress": "complete",
            "region": "US",
            "urn": "test-urn",
            "derivatives": [
                {
                    "status": "success",
                    "progress": "complete",
                    "outputType": "svf2",
                    "children": []
                }
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.derivatives.len(), 1);
        assert_eq!(manifest.derivatives[0].output_type, "svf2");
    }

    #[test]
    fn test_metadata_response_deserialization() {
        let json = r#"{
            "data": {
                "type": "metadata",
                "metadata": [
                    {
                        "guid": "abc-123",
                        "name": "3D View",
                        "role": "3d",
                        "mime": "application/autodesk-svf2",
                        "hasThumbnail": "true",
                        "progress": "complete"
                    }
                ]
            }
        }"#;
        let resp: MetadataResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.metadata.len(), 1);
        assert_eq!(resp.data.metadata[0].guid, "abc-123");
        assert_eq!(resp.data.metadata[0].role, "3d");
    }

    #[test]
    fn test_metadata_response_empty() {
        let json = r#"{"data": {"type": "metadata", "metadata": []}}"#;
        let resp: MetadataResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.metadata.is_empty());
    }

    #[test]
    fn test_object_tree_deserialization() {
        let json = r#"{
            "data": {
                "type": "objects",
                "objects": [
                    {
                        "objectid": 1,
                        "name": "Root",
                        "objects": [
                            {
                                "objectid": 2,
                                "name": "Child",
                                "objects": []
                            }
                        ]
                    }
                ]
            }
        }"#;
        let resp: ObjectTreeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.objects.len(), 1);
        assert_eq!(resp.data.objects[0].object_id, 1);
        assert_eq!(resp.data.objects[0].objects.len(), 1);
        assert_eq!(resp.data.objects[0].objects[0].name, "Child");
    }

    #[test]
    fn test_properties_response_deserialization() {
        let json = r#"{
            "data": {
                "type": "properties",
                "collection": [
                    {
                        "objectid": 42,
                        "name": "Wall",
                        "externalId": "ext-42",
                        "properties": {
                            "Dimensions": {
                                "Width": "300mm"
                            }
                        }
                    }
                ]
            }
        }"#;
        let resp: PropertiesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.collection.len(), 1);
        assert_eq!(resp.data.collection[0].object_id, 42);
        assert_eq!(resp.data.collection[0].name, "Wall");
        assert!(
            resp.data.collection[0]
                .properties
                .contains_key("Dimensions")
        );
    }

    #[test]
    fn test_property_query_by_object_ids() {
        let query = PropertyQuery::by_object_ids(vec![1, 2, 3]);
        let json = serde_json::to_value(&query).unwrap();
        let filter = &json["query"]["$in"];
        assert_eq!(filter[0], "objectid");
        assert_eq!(filter[1], 1);
        assert_eq!(filter[2], 2);
        assert_eq!(filter[3], 3);
        assert!(json.get("fields").is_none());
        assert!(json.get("pagination").is_none());
    }

    // ==================== Contract Tests ====================

    #[test]
    fn test_contract_manifest_success() {
        let json = include_str!("../../tests/fixtures/manifest_success.json");
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        insta::assert_json_snapshot!(manifest);
    }

    #[test]
    fn test_contract_manifest_pending() {
        let json = include_str!("../../tests/fixtures/manifest_pending.json");
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        insta::assert_json_snapshot!(manifest);
    }
}
