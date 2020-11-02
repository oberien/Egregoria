use crate::gui::{Tool, Z_TOOL};
use egregoria::engine_interaction::{KeyCode, KeyboardInfo, MouseButton, MouseInfo};
use egregoria::rendering::immediate::ImmediateDraw;
use egregoria::rendering::meshrender_component::MeshRender;
use geom::Color;
use geom::Spline;
use geom::Vec2;
use legion::system;
use map_model::{
    IntersectionID, LanePattern, LanePatternBuilder, Map, MapProject, ProjectKind, RoadSegmentKind,
};

#[derive(Clone, Copy, Debug)]
enum BuildState {
    Hover,
    Start(MapProject),
    Interpolation(Vec2, MapProject),
}

impl Default for BuildState {
    fn default() -> Self {
        BuildState::Hover
    }
}

#[derive(Default)]
pub struct RoadBuildResource {
    build_state: BuildState,
    pub pattern_builder: LanePatternBuilder,
}

#[system]
#[write_component(MeshRender)]
pub fn roadbuild(
    #[resource] state: &mut RoadBuildResource,
    #[resource] kbinfo: &KeyboardInfo,
    #[resource] mouseinfo: &MouseInfo,
    #[resource] tool: &Tool,
    #[resource] map: &mut Map,
    #[resource] immdraw: &mut ImmediateDraw,
) {
    if !matches!(*tool, Tool::RoadbuildStraight | Tool::RoadbuildCurved) {
        state.build_state = BuildState::Hover;
        return;
    }

    if kbinfo.just_pressed.contains(&KeyCode::Escape) {
        state.build_state = BuildState::Hover;
    }

    let cur_proj = map.project(mouseinfo.unprojected);

    state.update_drawing(immdraw, cur_proj.pos, state.pattern_builder.width());

    if mouseinfo.just_pressed.contains(&MouseButton::Left) {
        log::info!(
            "left clicked with state {:?} and {:?}",
            state.build_state,
            cur_proj.kind
        );
        use BuildState::*;
        use ProjectKind::*;

        // FIXME: Use or patterns when stable
        match (state.build_state, cur_proj.kind, *tool) {
            (Hover, ProjectKind::Ground, _)
            | (Hover, ProjectKind::Road(_), _)
            | (Hover, ProjectKind::Inter(_), _) => {
                // Hover selection
                state.build_state = BuildState::Start(cur_proj);
            }
            (Start(v), Ground, Tool::RoadbuildCurved) => {
                // Set interpolation point
                state.build_state = BuildState::Interpolation(mouseinfo.unprojected, v);
            }
            (Start(selected_proj), _, _) if compatible(map, cur_proj.kind, selected_proj.kind) => {
                // Straight connection to something
                let selected_after = make_connection(
                    map,
                    selected_proj,
                    cur_proj,
                    None,
                    &state.pattern_builder.build(),
                );

                let hover = MapProject {
                    pos: map.intersections()[selected_after].pos,
                    kind: ProjectKind::Inter(selected_after),
                };

                state.build_state = BuildState::Start(hover);
            }
            (Interpolation(interpoint, selected_proj), _, _)
                if compatible(map, cur_proj.kind, selected_proj.kind) =>
            {
                // Interpolated connection to something
                let selected_after = make_connection(
                    map,
                    selected_proj,
                    cur_proj,
                    Some(interpoint),
                    &state.pattern_builder.build(),
                );

                let hover = MapProject {
                    pos: map.intersections()[selected_after].pos,
                    kind: ProjectKind::Inter(selected_after),
                };

                state.build_state = BuildState::Start(hover);
            }
            _ => {}
        }
    }
}

fn make_connection(
    map: &mut Map,
    from: MapProject,
    to: MapProject,
    interpoint: Option<Vec2>,
    pattern: &LanePattern,
) -> IntersectionID {
    use ProjectKind::*;

    let connection_segment = match interpoint {
        Some(x) => RoadSegmentKind::from_elbow(from.pos, to.pos, x),
        None => RoadSegmentKind::Straight,
    };

    let mut mk_inter = |proj: MapProject| match proj.kind {
        Ground => map.add_intersection(proj.pos),
        Inter(id) => id,
        Road(id) => map.split_road(id, proj.pos),
        Building(_) | Lot(_) => unreachable!(),
    };

    let from = mk_inter(from);
    let to = mk_inter(to);

    map.connect(from, to, pattern, connection_segment);
    to
}

fn compatible(map: &Map, x: ProjectKind, y: ProjectKind) -> bool {
    use ProjectKind::*;
    match (x, y) {
        (Ground, Ground)
        | (Ground, Road(_))
        | (Ground, Inter(_))
        | (Road(_), Ground)
        | (Inter(_), Ground) => true,
        (Road(id), Road(id2)) => id != id2,
        (Inter(id), Inter(id2)) => id != id2,
        (Inter(id_inter), Road(id_road)) | (Road(id_road), Inter(id_inter)) => {
            let r = &map.roads()[id_road];
            r.src != id_inter && r.dst != id_inter
        }
        _ => false,
    }
}

impl RoadBuildResource {
    pub fn update_drawing(&self, immdraw: &mut ImmediateDraw, proj_pos: Vec2, patwidth: f32) {
        let transparent_blue = Color {
            r: 0.3,
            g: 0.3,
            b: 1.0,
            a: 1.0,
        };

        match self.build_state {
            BuildState::Hover => {
                immdraw
                    .circle(proj_pos, 2.0)
                    .color(transparent_blue)
                    .z(Z_TOOL);
            }
            BuildState::Start(x) => {
                immdraw
                    .circle(proj_pos, patwidth * 0.5)
                    .color(transparent_blue)
                    .z(Z_TOOL);
                immdraw
                    .circle(x.pos, patwidth * 0.5)
                    .color(transparent_blue)
                    .z(Z_TOOL);
                immdraw
                    .line(proj_pos, x.pos, patwidth)
                    .color(transparent_blue)
                    .z(Z_TOOL);
            }
            BuildState::Interpolation(p, x) => {
                let sp = Spline {
                    from: x.pos,
                    to: proj_pos,
                    from_derivative: (p - x.pos) * std::f32::consts::FRAC_1_SQRT_2,
                    to_derivative: (proj_pos - p) * std::f32::consts::FRAC_1_SQRT_2,
                };
                let mut points = sp.smart_points(1.0, 0.0, 1.0).peekable();
                while let Some(v) = points.next() {
                    immdraw
                        .circle(v, patwidth * 0.5)
                        .color(transparent_blue)
                        .z(Z_TOOL);

                    if let Some(peek) = points.peek() {
                        immdraw
                            .line(v, *peek, patwidth)
                            .color(transparent_blue)
                            .z(Z_TOOL);
                    }
                }
            }
        }
    }
}
