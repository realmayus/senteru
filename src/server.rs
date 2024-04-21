use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use osmpbfreader::{OsmId, RelationId};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::MapData;

pub type Db = Arc<RwLock<MapData>>;

pub async fn handler() -> &'static str {
    "Hello, World!"
}

pub async fn subway_lines(State(db): State<Db>) -> impl IntoResponse {
    let mut relations = db
        .read()
        .await
        .relations
        .values()
        .map(LineRel::from)
        .collect::<Vec<_>>();
    relations.sort_unstable_by_key(|rel| -(rel.len as isize));
    Json(relations)
}

#[derive(serde::Serialize)]
struct StopNode {
    id: i64,
    name: String,
    lat: i32,
    lon: i32,
}

#[derive(serde::Serialize)]
struct LineRel {
    id: i64,
    name: String,
    color: String,
    len: usize,
}

impl From<&osmpbfreader::Relation> for LineRel {
    fn from(relation: &osmpbfreader::Relation) -> Self {
        LineRel {
            id: relation.id.0,
            name: relation
                .tags
                .get("name:en")
                .map(|v| v.to_string())
                .or_else(|| relation.tags.get("name").map(|v| v.to_string()))
                .unwrap_or("".to_string()),
            color: relation
                .tags
                .get("colour")
                .map(|v| v.to_string())
                .or_else(|| relation.tags.get("color").map(|s| s.to_string()))
                .unwrap_or("".to_string()),
            len: relation.refs.len(),
        }
    }
}

#[derive(serde::Serialize)]
struct SubwayLine {
    relation: LineRel,
    nodes: Vec<StopNode>,
}

pub async fn subway_line(State(db): State<Db>, Path(id): Path<i64>) -> impl IntoResponse {
    let relation = db.read().await.relations.get(&RelationId(id)).cloned();
    let Some(relation) = relation else {
        return "Not found".into_response();
    };
    let _nodes = &db.read().await.nodes;
    let nodes = relation
        .refs
        .iter()
        .filter_map(|member| match member.member {
            OsmId::Node(node_id) => {
                let node = _nodes.get(&node_id);
                node.map(|node| StopNode {
                    id: node_id.0,
                    name: node
                        .tags
                        .get("name:en")
                        .map(|v| v.to_string())
                        .or_else(|| node.tags.get("name").map(|v| v.to_string()))
                        .unwrap_or("".to_string()),
                    lat: node.decimicro_lat,
                    lon: node.decimicro_lon,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    Json(SubwayLine {
        relation: (&relation).into(),
        nodes,
    })
    .into_response()
}
