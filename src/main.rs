mod server;

use axum::http::HeaderValue;

use axum::routing::get;
use axum::Router;
use bincode::{Decode, Encode};
use osmpbfreader::{Node, NodeId, OsmId, OsmPbfReader, Relation, RelationId, Way, WayId};
use server::Db;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[derive(Decode, Encode)]
struct MapData {
    #[bincode(with_serde)]
    nodes: BTreeMap<NodeId, osmpbfreader::Node>,
    #[bincode(with_serde)]
    ways: BTreeMap<WayId, osmpbfreader::Way>,
    #[bincode(with_serde)]
    relations: BTreeMap<RelationId, osmpbfreader::Relation>,
}

#[tokio::main]
async fn main() {
    let import_arg_present = std::env::args().any(|arg| arg == "--import");
    let mut data = if import_arg_present {
        let (include_nodes, include_ways, relations) =
            import(PathBuf::from("assets/Tokyo.osm.pbf"));

        let data = MapData {
            nodes: include_nodes,
            ways: include_ways,
            relations,
        };
        bincode::encode_into_std_write(
            &data,
            &mut std::fs::File::create("assets/Tokyo.bin").unwrap(),
            bincode::config::standard(),
        )
        .unwrap();
        data
    } else {
        bincode::decode_from_std_read(
            &mut std::fs::File::open("assets/Tokyo.bin").unwrap(),
            bincode::config::standard(),
        )
        .unwrap()
    };
    println!("Nodes: {:?}", data.nodes.len());
    println!("Ways: {:?}", data.ways.len());
    println!("Relations: {:?}", data.relations.len());

    // we only consider lines with more than 5 stops actual lines
    data.relations.retain(|_, rel| {
        rel.refs
            .iter()
            .filter(|rf| matches!(rf.member, OsmId::Node(_)))
            .count()
            > 5
    });
    // bucket of <usize, RelationId> which buckets all relations with the same number of nodes
    let mut buckets = BTreeMap::new();
    for (id, rel) in data.relations.iter() {
        let count = rel
            .refs
            .iter()
            .filter(|rf| matches!(rf.member, OsmId::Node(_)))
            .count();
        buckets.entry(count).or_insert_with(Vec::new).push(*id);
    }
    // discard lines that are practically the same (the second one)
    let mut to_remove = Vec::new();
    for (_, ids) in buckets.iter() {
        for (i, id) in ids.iter().enumerate() {
            for other_id in ids.iter().skip(i + 1) {
                let rel = data.relations.get(id).unwrap();
                let other_rel = data.relations.get(other_id).unwrap();
                if same_route(
                    &rel.refs
                        .iter()
                        .filter_map(|rf| match rf.member {
                            OsmId::Node(node_id) => Some(node_id),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    &other_rel
                        .refs
                        .iter()
                        .filter_map(|rf| match rf.member {
                            OsmId::Node(node_id) => Some(node_id),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    100,
                    &data.nodes,
                ) {
                    to_remove.push((*other_id, *id));
                }
            }
        }
    }
    let demote_keywords = ["bypass"];
    for (dupe_a, dupe_b) in to_remove {
        let original_id = if demote_keywords.iter().any(|&k| {
            data.relations[&dupe_a]
                .tags
                .get("name:en")
                .is_some_and(|v| v.to_lowercase().contains(k))
        }) {
            dupe_b
        } else {
            dupe_a
        };
        let del_id = if original_id == dupe_a {
            dupe_b
        } else {
            dupe_a
        };
        let deld = data.relations.remove(&del_id);
        println!(
            "Removing line {:?} as it's a duplicate of {:?}",
            deld.as_ref().map(|x| x
                .tags
                .get("name:en")
                .or_else(|| data.relations[&del_id].tags.get("name"))),
            data.relations.get(&original_id).map(|x| x
                .tags
                .get("name:en")
                .or_else(|| data.relations[&original_id].tags.get("name")))
        );
    }
    let exclude_keywords = ["bypass"];
    data.relations.retain(|_, rel| {
        if exclude_keywords.iter().any(|&k| {
            rel.tags
                .get("name:en")
                .is_some_and(|v| v.to_lowercase().contains(k))
        }) {
            println!(
                "Excluding line {:?} because of exclusion keyword",
                rel.tags.get("name:en").or_else(|| rel.tags.get("name"))
            );
            false
        } else {
            true
        }
    });

    println!("Number of remaining lines: {:?}", data.relations.len());

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new()
        .route("/", get(server::handler))
        .route("/subway_lines", get(server::subway_lines))
        .route("/subway_lines/:id", get(server::subway_line))
        .layer(cors)
        .with_state(Db::new(RwLock::new(data)));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn import(
    path: PathBuf,
) -> (
    BTreeMap<NodeId, Node>,
    BTreeMap<WayId, Way>,
    BTreeMap<RelationId, Relation>,
) {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = OsmPbfReader::new(file);
    let mut nodes = BTreeMap::new();
    let mut include_nodes = BTreeMap::new();
    let mut ways = BTreeMap::new();
    let mut include_ways = BTreeMap::new();
    let mut relations = BTreeMap::new();
    reader.par_iter().for_each(|obj| {
        if let Ok(obj) = obj {
            match obj {
                osmpbfreader::OsmObj::Node(node) => {
                    nodes.insert(node.id, node);
                }
                osmpbfreader::OsmObj::Way(way) => {
                    ways.insert(way.id, way);
                }
                _ => {}
            }
        }
    });
    reader.rewind().unwrap();
    reader.par_iter().for_each(|obj| {
        if let Ok(obj) = obj {
            if let osmpbfreader::OsmObj::Relation(relation) = obj {
                if relation.tags.contains("route", "subway") {
                    relation.refs.iter().for_each(|member| match member.member {
                        OsmId::Node(node_id) => {
                            if let Some(node) = nodes.get(&node_id) {
                                include_nodes.insert(node_id, node.clone());
                            } else {
                                println!("Ref node not found: {:?}", node_id);
                            }
                        }
                        OsmId::Way(way_id) => {
                            if let Some(way) = ways.get(&way_id) {
                                include_ways.insert(way_id, way.clone());
                            } else {
                                println!("Ref way not found: {:?}", way_id);
                            }
                        }
                        _ => {}
                    });
                    relations.insert(relation.id, relation);
                }
            }
        }
    });
    (include_nodes, include_ways, relations)
}

fn same_route(
    nodes_a: &[NodeId],
    nodes_b: &[NodeId],
    tolerance_meters: i32,
    all_nodes: &BTreeMap<NodeId, Node>,
) -> bool {
    if nodes_a.len() != nodes_b.len() {
        return false;
    }
    let res = nodes_a.iter().zip(nodes_b.iter()).all(|(node_a, node_b)| {
        let node_a = all_nodes.get(node_a).unwrap();
        let node_b = all_nodes.get(node_b).unwrap();
        almost_same_position(
            (node_a.decimicro_lat, node_a.decimicro_lon),
            (node_b.decimicro_lat, node_b.decimicro_lon),
            tolerance_meters,
        )
    }) || nodes_a
        .iter()
        .zip(nodes_b.iter().rev())
        .all(|(node_a, node_b)| {
            let node_a = all_nodes.get(node_a).unwrap();
            let node_b = all_nodes.get(node_b).unwrap();
            almost_same_position(
                (node_a.decimicro_lat, node_a.decimicro_lon),
                (node_b.decimicro_lat, node_b.decimicro_lon),
                tolerance_meters,
            )
        });
    if res {
        println!("Same route: {:?} and {:?}", nodes_a, nodes_b);
    }
    res
}

fn almost_same_position(latlon_a: (i32, i32), latlon_b: (i32, i32), tolerance_meters: i32) -> bool {
    let lat_diff = latlon_a.0 - latlon_b.0;
    let lon_diff = latlon_a.1 - latlon_b.1;
    let lat_diff_m = lat_diff as f64 * 1e-7 * 111_111.0; // 1 degree latitude is 111,111 meters
    let lon_diff_m = lon_diff as f64 * 1e-7 * 111_111.0 * (latlon_a.0 as f64).to_radians().cos(); // 1 degree longitude is 111,111 meters times cos(latitude)
    lat_diff_m * lat_diff_m + lon_diff_m * lon_diff_m
        < tolerance_meters as f64 * tolerance_meters as f64
}
