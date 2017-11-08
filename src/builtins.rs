use ::*;

/// Sends input params down to children and events back to the parent node.
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

    fn end(&mut self, _ctx: &mut Context<E, P, Self>) { }
}

/// Bounces input params back as events.
pub struct Bounce;
impl<A: Clone> Node<A, A> for Bounce {
    type Event = ();
    type Output = ();

    fn update(&mut self, ctx: &mut Context<A, A, Self>) {
        ctx.send(ctx.param.clone())
    }

    fn event(&mut self, _ctx: &mut Context<A, A, Self>, _event: ()) { }

    fn end(&mut self, _ctx: &mut Context<A, A, Self>) { }
}
