use std::mem::{replace};
use std::marker::PhantomData;
use std::any::Any;

pub struct TreeBuilder<'a, E, P> {
    nodes: &'a mut Vec<Item>,
    _phantom: PhantomData<(E, P)>,
}

impl<'a, E, P> TreeBuilder<'a, E, P> {
    pub fn push_boxed<N: Node<E, P> + 'static>(&mut self, node: Box<N>) -> TreeBuilder<N::Event, N::Output> {
        self.nodes.push(Item {
            node: node as Box<Any>,
            apply: apply::<E, P, N>,
            children: Vec::with_capacity(0),
            live: true,
        });
        TreeBuilder {
            nodes: &mut self.nodes.last_mut().unwrap().children,
            _phantom: PhantomData,
        }
    }

    pub fn push<N: Node<E, P> + 'static>(&mut self, node: N) -> TreeBuilder<N::Event, N::Output> {
        self.push_boxed(Box::new(node))
    }

    pub fn append(&mut self, mut tree: Tree<E, P>) {
        self.nodes.append(&mut tree.nodes);
    }
}

pub struct Tree<E, P> {
    nodes: Vec<Item>,
    pub events: Vec<E>,
    _phantom: PhantomData<P>,
}

impl<E, P> Tree<E, P> {
    pub fn new() -> Tree<E, P> {
        Tree {
            nodes: Vec::new(),
            events: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn build(&mut self) -> TreeBuilder<E, P> {
        TreeBuilder {
            nodes: &mut self.nodes,
            _phantom: PhantomData,
        }
    }

    pub fn push<N: Node<E, P> + 'static>(&mut self, node: N) {
        self.build().push(node);
    }

    pub fn update(&mut self, param: &P) {
        let send = &mut self.events as *mut _ as *mut ();
        let param = param as *const _  as *const ();
        let mut add = Vec::with_capacity(0);

        for n in self.nodes.iter_mut().filter(|n| n.live) {
            (n.apply)(n, send, param, &mut add);
        }

        self.nodes.append(&mut add);
    }
}

struct Item {
    node: Box<Any>,
    apply: fn(&mut Item, *mut (), *const (), &mut Vec<Item>),
    children: Vec<Item>,
    live: bool,
}

fn apply<S, P, N: Node<S, P> + 'static>(item: &mut Item, send: *mut (), param: *const (), sib: &mut Vec<Item>) {
    let mut ctx = Context {
        send: unsafe { &mut *(send as *mut Vec<S>) },
        param: unsafe { &*(param as *const P) },
        events: Vec::with_capacity(0),
        chi: Vec::with_capacity(0),
        sib: sib,
        live: item.live,
    };

    let node = (&mut *item.node).downcast_mut::<N>().expect("Node type mismatch");
    let mut o = node.update(&mut ctx);

    for mut c in &mut item.children.iter_mut().filter(|c| c.live) {
        (c.apply)(
            &mut c,
            &mut ctx.events as *mut _ as *mut (),
            &mut o as *const _ as *const (),
            &mut ctx.chi);
    }

    while !ctx.events.is_empty() {
        for e in replace(&mut ctx.events, Vec::with_capacity(0)) {
            node.event(&mut ctx, e);
        }
    }

    node.post(&mut ctx);

    if ctx.live {
        item.children.append(&mut ctx.chi);
    } else {
        item.live = false;
        item.children.clear();
    }
}

pub struct Context<'a, S: 'a, P: 'a, N: 'a + Node<S, P> + ?Sized> {
    send: &'a mut Vec<S>,
    param: &'a P,
    events: Vec<N::Event>,
    chi: Vec<Item>,
    sib: &'a mut Vec<Item>,
    live: bool,
}

impl<'a, S, P, N: Node<S, P>> Context<'a, S, P, N> {
    #[inline]
    pub fn send(&mut self, event: S) {
        self.send.push(event);
    }

    #[inline]
    pub fn send_all<I: Iterator<Item=S>>(&mut self, events: I) {
        self.send.extend(events);
    }

    #[inline]
    pub fn accept(&mut self, event: N::Event) {
        self.events.push(event);
    }

    #[inline]
    pub fn accept_all<I: Iterator<Item=N::Event>>(&mut self, events: I) {
        self.events.extend(events);
    }

    #[inline]
    pub fn children(&mut self) -> TreeBuilder<N::Event, N::Output> {
        TreeBuilder {
            nodes: &mut self.chi,
            _phantom: PhantomData,
        }
    }
    
    #[inline]
    pub fn siblings(&mut self) -> TreeBuilder<S, P> {
        TreeBuilder {
            nodes: self.sib,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn kill(&mut self) {
        self.live = false;
    }

    #[inline]
    pub fn revive(&mut self) {
        self.live = true;
    }

    #[inline]
    pub fn param(&self) -> &P {
        self.param
    }
}

pub trait Node<S, P> {
    type Output;
    type Event;

    fn update(&mut self, ctx: &mut Context<S, P, Self>) -> Self::Output;
    fn event(&mut self, ctx: &mut Context<S, P, Self>, event: Self::Event);
    fn post(&mut self, _ctx: &mut Context<S, P, Self>) {}
}

#[macro_export]
macro_rules! node_funcs {
    ($S:ty, $P:ty, $ctx:ident, $(
        $(update $update_pat:pat => $update_expr:expr,)*
        $(post $post_pat:pat => $post_expr:expr,)*
        $(event $event_pat:pat => $event_expr:expr),*,
    )*$(,)*) => {
        #[allow(unused)]
        fn update(&mut self, $ctx: &mut $crate::Context<$S, $P, Self>) -> Self::Output {
            match *$ctx.param {
                $($($update_pat => $update_expr,)*)*
            }
        }

        #[allow(unused)]
        fn event(&mut self, $ctx: &mut $crate::Context<$S, $P, Self>, event: Self::Event) {
            match event {
                $($($event_pat => { $event_expr; },)*)*
                _ => (),
            }
        }

        #[allow(unused)]
        fn post(&mut self, $ctx: &mut $crate::Context<$S, $P, Self>) {
            match *$ctx.param {
                $($($post_pat => { $post_expr; },)*)*
                _ => (),
            }
        }
    };
}

#[macro_export]
macro_rules! node_impl {
    {
        impl$(<$($($gen:tt):*),+>)* Node<$S:ty, $P:ty> for $ty:path $(where $($wh:tt),*)*{
            type Event = $event:ty;
            type Output = $output:ty;
            
            match $ctx:ident {
                $($m:ident $p:pat => $e:expr),*
                $(,)*
            }
        }
    } => {
        impl$(<$($($gen):*),+>)* $crate::Node<$S, $P> for $ty $(where $($wh),*)* {
            type Event = $event;
            type Output = $output;

            node_funcs!($S, $P, $ctx, $($m $p => $e,)*);
        }
    }
}

pub struct Passthrough;
node_impl! {
    impl<E, P: Clone> Node<E, P> for Passthrough {
        type Event = E;
        type Output = P;

        match ctx {
            update ref param => param.clone(),
            event e => ctx.send(e),
        }
    }
}
