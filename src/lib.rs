#![feature(box_syntax, core_intrinsics)]

pub mod updown {
    use std::mem::{replace, uninitialized};
    use std::ptr::write;
    use std::marker::PhantomData;
    use std::any::Any;
    use std::intrinsics::type_name;

    pub struct TreeBuilder<'a, E, P> {
        nodes: &'a mut Vec<Item>,
        _phantom: PhantomData<(E, P)>,
    }

    impl<'a, E, P> TreeBuilder<'a, E, P> {
        pub fn push<N: Node<E, P> + 'static>(&mut self, node: N) -> TreeBuilder<N::Event, N::Output> {
            self.nodes.push(Item {
                node: raw_node(node),
                update: update::<E, P, N::Event, N::Output>,
                children: Vec::with_capacity(0),
                live: true,
            });
            TreeBuilder {
                nodes: &mut self.nodes.last_mut().unwrap().children,
                _phantom: PhantomData,
            }
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

            for n in &mut self.nodes {
                let up = n.update;
                up(n, send, param, &mut add);
            }

            self.nodes.append(&mut add);
        }
    }

    struct Item {
        node: RawNode,
        update: fn(&mut Item, *mut (), *const (), &mut Vec<Item>),
        children: Vec<Item>,
        live: bool,
    }

    fn update<S, P, E, O>(item: &mut Item, send: *mut (), param: *const (), sib: &mut Vec<Item>) {
        let mut ctx = Context {
            send: unsafe { &mut *(send as *mut Vec<S>) },
            param: unsafe { &*(param as *const P) },
            events: Vec::<E>::with_capacity(0),
            chi: Vec::with_capacity(0),
            sib: sib,
            _phantom: PhantomData::<O>,
        };
        let mut o = unsafe { uninitialized::<O>() };
        (item.node.update)(
            &mut *item.node.data,
            &mut ctx as *mut _ as *mut (),
            &mut o as *mut _ as *mut ());

        for mut c in &mut item.children.iter_mut().filter(|c| c.live) {
            (c.update)(
                &mut c,
                &mut ctx.events as *mut _ as *mut (),
                &mut o as *mut _ as *mut (),
                &mut ctx.chi);
        }

        for e in replace(&mut ctx.events, Vec::with_capacity(0)) {
            (item.node.event)(
                &mut *item.node.data,
                &mut ctx as *mut _ as *mut (),
                &e as *const _  as *const ())
        }

        item.children.append(&mut ctx.chi);
    }

    pub struct Context<'a, S: 'a, P: 'a, E: 'a, O: 'a> {
        send: &'a mut Vec<S>,
        pub param: &'a P,
        events: Vec<E>,
        chi: Vec<Item>,
        sib: &'a mut Vec<Item>,
        _phantom: PhantomData<O>,
    }

    impl<'a, S, P, E, O> Context<'a, S, P, E, O> {
        pub fn send(&mut self, event: S) {
            self.send.push(event);
        }

        pub fn accept(&mut self, event: E) {
            self.events.push(event);
        }

        pub fn child<N: Node<O, E> + 'static>(&mut self, node: N) {
            self.chi.push(Item {
                node: raw_node(node),
                update: update::<O, E, N::Event, N::Output>,
                children: Vec::with_capacity(0),
                live: true,
            });
        }
        
        pub fn sibling<N: Node<S, P> + 'static>(&mut self, node: N) {
            self.sib.push(Item {
                node: raw_node(node),
                update: update::<S, P, N::Event, N::Output>,
                children: Vec::with_capacity(0),
                live: true,
            });
        }
    }

    pub trait Node<S, P> {
        type Output;
        type Event;

        fn update(&mut self, ctx: &mut Context<S, P, Self::Event, Self::Output>) -> Self::Output;
        fn event(&mut self, ctx: &mut Context<S, P, Self::Event, Self::Output>, event: &Self::Event);
    }

    pub struct RawNode {
        data: Box<Any>,
        update: fn(&mut Any, *mut (), *mut ()),
        event: fn(&mut Any, *mut (), *const ()),
    }

    fn raw_node<N: Node<S, P> + 'static, S, P>(n: N) -> RawNode {
        fn update<N: Node<S, P> + 'static, S, P>(n: &mut Any, ctx: *mut (), out: *mut ()) {
            unsafe {
                write(out as *mut N::Output, Node::update(
                    n.downcast_mut::<N>().unwrap(),
                    &mut *(ctx as *mut Context<S, P, N::Event, N::Output>),
                ));
            }
        }

        fn event<N: Node<S, P> + 'static, S, P>(n: &mut Any, ctx: *mut (), event: *const ()) {
            unsafe {
                Node::event(
                    n.downcast_mut::<N>().unwrap(),
                    &mut *(ctx as *mut Context<S, P, N::Event, N::Output>),
                    &*(event as *const N::Event),
                );
            }
        }

        RawNode {
            data: Box::new(n),
            update: update::<N, S, P>,
            event:event::<N, S, P>,
        }
    }
}

use updown::*;

struct Echo;

impl<E: Clone> Node<E, E> for Echo {
    type Event = E;
    type Output = ();
    
    fn update(&mut self, ctx: &mut Context<E, E, E, ()>) {
        ctx.send(ctx.param.clone())
    }

    fn event(&mut self, ctx: &mut Context<E, E, E, ()>, e: &E) {
        ctx.send(e.clone())
    }
}

struct Append {
    append: String,
}

impl<E: Clone> Node<E, String> for Append {
    type Event = E;
    type Output = String;

    fn update(&mut self, ctx: &mut Context<E, String, E, String>) -> String {
        (*ctx.param).to_owned() + &self.append
    }

    fn event(&mut self, ctx: &mut Context<E, String, E, String>, e: &E) {
        ctx.send(e.clone());
    }
}

fn main() {
    let mut tree: Tree<String, String> = Tree::new();
    tree.build()
        .push(Append { append: "World".to_owned() })
        .push(Echo);
    tree.update(&"Hello ".to_owned());
}
