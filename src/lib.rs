#![feature(box_syntax, box_patterns)]

use std::marker::PhantomData;
use std::any::Any;

struct Item {
    node: Box<RawNode>,
    tree: ItemTree,
}

struct ItemTree {
    live: bool,
    children: Vec<Item>,
}

pub struct World<E, P> {
    nodes: Vec<Item>,
    pub events: Vec<E>,
    _phantom: PhantomData<P>,
}

impl<E: 'static, P: 'static> World<E, P> {
    pub fn new() -> World<E, P> {
        World {
            nodes: Vec::new(),
            events: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn push<N>(&mut self, node: N) 
        where N: Node<Param=P, Generate=E> + RawNode + 'static
    {
        self.nodes.push(Item {
            node: box node,
            tree: ItemTree {
                live: true,
                children: Vec::with_capacity(0),
            },
        });
    }

    pub fn update(&mut self, param: P) {
        let param = &param as &Any;
        let mut siblings = Vec::with_capacity(0);
        let mut new_events = Vec::with_capacity(0);
        for c in self.nodes.iter_mut().filter(|c| c.tree.live) {
            let mut ctx = Context {
                t: &mut c.tree,
                sib: &mut siblings,
                gen_events: &mut new_events,
                my_events: Vec::with_capacity(0),
                _phantom: PhantomData,
            };
            unsafe {
                c.node.update(&mut ctx, param);
                while !ctx.my_events.is_empty() {
                    for e in ::std::mem::replace(&mut ctx.my_events, Vec::with_capacity(0)) {
                        c.node.receive(&mut ctx, &*e);
                    }
                }
            }
        }
        self.nodes.append(&mut siblings);
        self.events.extend(new_events.into_iter().map(|e| 
            if let Ok(box e) = e.downcast() { e } else { panic!("N::Generate != E") }
        ));
    }
}

pub struct Context<'a, N> {
    t: &'a mut ItemTree,
    sib: &'a mut Vec<Item>,
    gen_events: &'a mut Vec<Box<Any>>, // TODO: Does not need to box every event
    my_events: Vec<Box<Any>>,
    _phantom: PhantomData<N>,
}

impl<'a, N: Node> Context<'a, N> {
    /// Mark self as dead, instantly dropping all children.
    pub fn kill(&mut self) {
        self.t.live = false;
        self.t.children.clear();
    }

    /// Add a sibling to self.
    pub fn sibling<S>(&mut self, sib: S) 
        where S: Node<Param=N::Param, Generate=N::Generate> + RawNode + 'static
    {
        self.sib.push(Item {
            node: box sib,
            tree: ItemTree {
                live: true,
                children: Vec::with_capacity(0),
            },
        });
    }
    
    /// Replace self, instantly dropping all children.
    pub fn replace<S>(&mut self, with: S) 
        where S: Node<Param=N::Param, Generate=N::Generate> + RawNode + 'static
    {
        self.kill();
        self.sibling(with);
    }

    /// Add a child of self.
    pub fn child<C>(&mut self, child: C)
        where C: Node<Param=N::Output, Generate=N::Event> + RawNode + 'static
    {
        self.t.children.push(Item {
            node: box child,
            tree: ItemTree {
                live: true,
                children: Vec::with_capacity(0),
            },
        });
    }

    /// Send event to self.
    pub fn apply(&mut self, event: N::Event) {
        self.my_events.push(box event);
    }

    /// Send many events to self.
    pub fn apply_all<I: Iterator<Item=N::Event>>(&mut self, events: I) {
        self.my_events.extend(events.map(|e| box e as Box<Any>));
    }

    /// Send event to parent.
    pub fn send(&mut self, event: N::Generate) {
        self.gen_events.push(box event);
    }

    /// Send many events to parent.
    pub fn send_all<I: Iterator<Item=N::Generate>>(&mut self, events: I) {
        self.gen_events.extend(events.map(|e| box e as Box<Any>));
    }

    /// Sends an update to children.
    pub fn update(&mut self, param: N::Output) {
        let param = &param as &Any;
        let mut siblings = Vec::with_capacity(0);
        for c in self.t.children.iter_mut().filter(|c| c.tree.live) {
            let mut ctx = Context {
                t: &mut c.tree,
                sib: &mut siblings,
                gen_events: &mut self.my_events,
                my_events: Vec::with_capacity(0),
                _phantom: PhantomData,
            };
            unsafe {
                c.node.update(&mut ctx, param);
                while !ctx.my_events.is_empty() {
                    for e in ::std::mem::replace(&mut ctx.my_events, Vec::with_capacity(0)) {
                        c.node.receive(&mut ctx, &*e);
                    }
                }
            }
        }
        self.t.children.append(&mut siblings);
    }
}

pub trait Node: Sized {
    type Event: 'static;
    type Generate: 'static;

    type Param: 'static;
    type Output: 'static;

    fn update(&mut self, ctx: &mut Context<Self>, par: &Self::Param);
    fn receive(&mut self, ctx: &mut Context<Self>, ev: &Self::Event);
}

pub trait RawNode {
    unsafe fn update(&mut self, ctx: *mut Context<()>, par: &Any);
    unsafe fn receive(&mut self, ctx: *mut Context<()>, ev: &Any);
}

impl<N: Node> RawNode for N {
    unsafe fn update(&mut self, ctx: *mut Context<()>, par: &Any) {
        Node::update(self, &mut *(ctx as *mut Context<N>),
            par.downcast_ref().expect("N::Output != C::Param")
        );
    }

    unsafe fn receive(&mut self, ctx: *mut Context<()>, ev: &Any) {
        Node::receive(self, &mut *(ctx as *mut Context<N>),
            ev.downcast_ref().expect("N::Event != C::Generate")
        );
    }
}

struct Appender {
    append: String,
}

impl<E> Node for Appender {
    type Generate = E;
    type Event = E;
    type Param = String;
    type Output = String;

    fn update(&mut self, ctx: &mut Context<Appender>, par: String) {
        ctx.update(par + &self.append);
    }

    fn receive(&mut self, ctx: &mut Context<Appender>, event: E) {
        ctx.send(event);
    }
}

fn main() {
    let mut world: World<String, String> = World::new();
    world.push()
}
