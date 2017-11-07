use ::*;

/// Sends input params and events out to the parent node.
pub struct Passthrough;
impl<E, P: Clone> Node<E, P> for Passthrough {
    type Event = E;
    type Output = P;

    fn update(&mut self, ctx: &mut Context<E, P, Self>) -> P {
        return ctx.param.clone()
    }

    fn event(&mut self, ctx: &mut Context<E, P, Self>, event: E) {
        ctx.send(event)
    }

    fn post(&mut self, _ctx: &mut Context<E, P, Self>) { }
}
