use common::AudioKind;
use geom::{LinearColor, Polygon, Vec2, OBB};

register_resource_noserialize!(ImmediateSound);
#[derive(Default)]
pub struct ImmediateSound {
    pub orders: Vec<(&'static str, AudioKind)>,
}

impl ImmediateSound {
    pub fn play(&mut self, sound: &'static str, kind: AudioKind) {
        self.orders.push((sound, kind))
    }
}

#[derive(Clone)]
pub enum OrderKind {
    Circle {
        pos: Vec2,
        radius: f32,
    },
    StrokeCircle {
        pos: Vec2,
        radius: f32,
        thickness: f32,
    },
    Line {
        from: Vec2,
        to: Vec2,
        thickness: f32,
    },
    PolyLine {
        points: Vec<Vec2>,
        thickness: f32,
    },
    Polygon {
        poly: Polygon,
    },
    OBB(OBB),
}

#[derive(Clone)]
pub struct ImmediateOrder {
    pub kind: OrderKind,
    pub color: LinearColor,
    pub z: f32,
}

register_resource_noserialize!(ImmediateDraw);
#[derive(Default)]
pub struct ImmediateDraw {
    pub orders: Vec<ImmediateOrder>,
    pub persistent_orders: Vec<ImmediateOrder>,
}

pub struct ImmediateBuilder<'a> {
    draw: &'a mut ImmediateDraw,
    order: ImmediateOrder,
    persistent: bool,
}

impl<'a> ImmediateBuilder<'a> {
    pub fn color(&mut self, col: impl Into<LinearColor>) -> &mut Self {
        self.order.color = col.into();
        self
    }

    pub fn z(&mut self, z: f32) -> &mut Self {
        self.order.z = z;
        self
    }

    pub fn persistent(&mut self) -> &mut Self {
        self.persistent = true;
        self
    }
}

impl<'a> Drop for ImmediateBuilder<'a> {
    fn drop(&mut self) {
        let order = std::mem::replace(
            &mut self.order,
            ImmediateOrder {
                kind: OrderKind::Circle {
                    pos: Vec2::ZERO,
                    radius: 0.0,
                },
                color: LinearColor::TRANSPARENT,
                z: 0.0,
            },
        );
        if self.persistent {
            self.draw.persistent_orders.push(order)
        } else {
            self.draw.orders.push(order)
        }
    }
}

impl ImmediateDraw {
    fn builder(&mut self, kind: OrderKind) -> ImmediateBuilder {
        ImmediateBuilder {
            draw: self,
            order: ImmediateOrder {
                kind,
                color: LinearColor::WHITE,
                z: 3.0,
            },
            persistent: false,
        }
    }
    pub fn circle(&mut self, pos: Vec2, radius: f32) -> ImmediateBuilder {
        self.builder(OrderKind::Circle { pos, radius })
    }

    pub fn line(&mut self, from: Vec2, to: Vec2, thickness: f32) -> ImmediateBuilder {
        self.builder(OrderKind::Line {
            from,
            to,
            thickness,
        })
    }

    pub fn polyline(&mut self, points: impl Into<Vec<Vec2>>, thickness: f32) -> ImmediateBuilder {
        self.builder(OrderKind::PolyLine {
            points: points.into(),
            thickness,
        })
    }

    pub fn polygon(&mut self, poly: Polygon) -> ImmediateBuilder {
        self.builder(OrderKind::Polygon { poly })
    }

    pub fn stroke_circle(&mut self, pos: Vec2, radius: f32, thickness: f32) -> ImmediateBuilder {
        self.builder(OrderKind::StrokeCircle {
            pos,
            radius,
            thickness,
        })
    }

    pub fn obb(&mut self, obb: OBB) -> ImmediateBuilder {
        self.builder(OrderKind::OBB(obb))
    }

    pub fn clear_persistent(&mut self) {
        self.persistent_orders.clear();
    }
}
