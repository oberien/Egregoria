use super::{Intersection, RoadNode};
use crate::cars::roads::TrafficLight::Always;
use crate::cars::roads::{TrafficLight, TrafficLightSchedule};
use crate::cars::IntersectionComponent;
use crate::graphs::graph::{Edge, Graph, NodeID};
use crate::interaction::{Movable, Selectable};
use crate::physics::physics_components::Transform;
use crate::rendering::meshrender_component::{CircleRender, MeshRender};
use crate::rendering::RED;
use cgmath::Vector2;
use cgmath::{InnerSpace, MetricSpace};
use specs::{Entities, WriteStorage};

pub struct RoadGraph {
    nodes: Graph<RoadNode>,
    intersections: Graph<Intersection>,
}

impl RoadGraph {
    pub fn new() -> RoadGraph {
        RoadGraph {
            intersections: Graph::new(),
            nodes: Graph::new(),
        }
    }

    pub fn nodes(&self) -> &Graph<RoadNode> {
        &self.nodes
    }
    pub fn intersections(&self) -> &Graph<Intersection> {
        &self.intersections
    }

    pub fn set_node_position(&mut self, i: &NodeID, pos: Vector2<f32>) {
        self.nodes.get_mut(i).map(|x| x.pos = pos);
    }

    pub fn set_intersection_position(&mut self, i: &NodeID, pos: Vector2<f32>) {
        self.intersections.get_mut(i).map(|x| x.pos = pos);
    }

    pub fn add_intersection(&mut self, i: Intersection) -> NodeID {
        self.intersections.push(i)
    }

    pub fn update_traffic_lights(&mut self, i: &NodeID) {
        let inter = &self.intersections[i];
        if inter.in_nodes.len() <= 2 {
            for (_, id) in inter.in_nodes.iter() {
                self.nodes[id].light = Always;
            }
            return;
        }
        let cycle_size = 10;
        for (i, (_, id)) in inter.in_nodes.iter().enumerate() {
            self.nodes[id].light = TrafficLight::Periodic(TrafficLightSchedule::from_basic(
                cycle_size,
                3,
                cycle_size + 3,
                if i % 2 == 0 { cycle_size + 3 } else { 0 },
            ));
        }
    }

    pub fn calculate_nodes_positions(&mut self, i: &NodeID) {
        let inter = &self.intersections[i];
        let center = inter.pos;

        for (to, node_id) in &inter.out_nodes {
            let inter2 = &self.intersections[to];
            let center2 = inter2.pos;

            let diff = center2 - center;
            let inter_length = diff.magnitude().max(1e-8);
            let dir = (center2 - center) / inter_length;

            let inter_length = (inter_length / 2.0).min(25.0);

            let nor: Vector2<f32> = Vector2::new(-dir.y, dir.x);

            let rn = self.nodes.get_mut(node_id).unwrap();
            rn.pos = center + dir * inter_length - nor * 4.0;

            let rn2 = self.nodes.get_mut(&inter2.in_nodes[&i]).unwrap();
            rn2.pos = center2 - dir * inter_length - nor * 4.0;
        }

        for (to, node_id) in &inter.in_nodes {
            let inter2 = &self.intersections[to];
            let center2 = inter2.pos;

            let diff = center2 - center;
            let inter_length = diff.magnitude();
            let dir = (center2 - center) / inter_length;

            let inter_length = (inter_length / 2.0).min(25.0);

            let nor: Vector2<f32> = Vector2::new(-dir.y, dir.x);

            let rn = self.nodes.get_mut(node_id).unwrap();
            rn.pos = center + dir * inter_length + nor * 4.0;

            let rn2 = self.nodes.get_mut(&inter2.out_nodes[&i]).unwrap();
            rn2.pos = center2 - dir * inter_length + nor * 4.0;
        }
    }

    pub fn make_entity<'a>(
        &mut self,
        inter_id: &NodeID,
        entities: &Entities<'a>,
        inters: &mut WriteStorage<'a, IntersectionComponent>,
        meshr: &mut WriteStorage<'a, MeshRender>,
        transforms: &mut WriteStorage<'a, Transform>,
        movable: &mut WriteStorage<'a, Movable>,
        selectable: &mut WriteStorage<'a, Selectable>,
    ) {
        let inter: &mut Intersection = &mut self.intersections[inter_id];
        inter.e = Some(
            entities
                .build_entity()
                .with(IntersectionComponent { id: *inter_id }, inters)
                .with(
                    MeshRender::simple(
                        CircleRender {
                            radius: 2.0,
                            color: RED,
                            filled: true,
                            ..CircleRender::default()
                        },
                        2,
                    ),
                    meshr,
                )
                .with(Transform::new(inter.pos), transforms)
                .with(Movable, movable)
                .with(Selectable, selectable)
                .build(),
        );
    }

    pub fn closest_node(&self, pos: Vector2<f32>) -> NodeID {
        let mut id: NodeID = *self.nodes.ids().next().unwrap();
        let mut min_dist = self.nodes.get(&id).unwrap().pos.distance2(pos);

        for (key, value) in &self.nodes {
            let dist = pos.distance2(value.pos);
            if dist < min_dist {
                id = *key;
                min_dist = dist;
            }
        }
        id
    }

    pub fn delete_inter(&mut self, id: &NodeID, entities: &Entities) {
        for Edge { to, .. } in self.intersections.get_neighs(id).clone() {
            self.disconnect_directional(id, &to);
        }
        for Edge { to, .. } in self.intersections.get_backward_neighs(id).clone() {
            self.disconnect_directional(&to, id);
        }
        self.intersections[&id].e.map(|x| entities.delete(x));
        self.intersections.remove_node(id);
    }

    pub fn disconnect(&mut self, a: &NodeID, b: &NodeID) {
        self.disconnect_directional(a, b);
        self.disconnect_directional(b, a);
    }

    pub fn disconnect_directional(&mut self, from: &NodeID, to: &NodeID) {
        self.intersections.remove_neigh(from, to);
        let inter_from_node = &self.intersections[from].out_nodes[to];
        let inter_to_node = &self.intersections[to].in_nodes[from];

        self.nodes.remove_node(inter_from_node);
        self.nodes.remove_node(inter_to_node);

        self.intersections
            .get_mut(from)
            .unwrap()
            .out_nodes
            .remove(to);

        self.intersections
            .get_mut(to)
            .unwrap()
            .in_nodes
            .remove(from);
        self.update_traffic_lights(to);
    }

    pub fn connect(&mut self, a: &NodeID, b: &NodeID) {
        self.connect_directional(a, b);
        self.connect_directional(b, a);
    }

    pub fn connect_directional(&mut self, from: &NodeID, to: &NodeID) {
        if self.intersections[from].pos == self.intersections[to].pos {
            println!("Couldn't connect two intersections because they are at the same place.");
            return;
        }
        self.intersections.add_neigh(from, to, 1.0);

        let rn_out = RoadNode::new([0.0, 0.0].into());
        let rn_in = RoadNode::new([0.0, 0.0].into());

        let out_id = self.nodes.push(rn_out);
        let in_id = self.nodes.push(rn_in);
        self.nodes.add_neigh(&out_id, &in_id, 0.0);

        let inter = self.intersections.get_mut(&from).unwrap();
        inter.out_nodes.insert(*to, out_id);
        for (from_id, in_id) in &inter.in_nodes {
            if from_id == to {
                continue;
            }
            self.nodes.add_neigh(in_id, &out_id, 1.0); // FIXME: Use actual internal road length
        }

        let inter2 = self.intersections.get_mut(&to).unwrap();
        inter2.in_nodes.insert(*from, in_id);
        for (to_id, out) in &inter2.out_nodes {
            if to_id == from {
                continue;
            }
            self.nodes.add_neigh(&in_id, out, 1.0);
        }

        self.calculate_nodes_positions(from);
        self.update_traffic_lights(to);
    }
}