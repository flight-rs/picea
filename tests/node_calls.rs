extern crate picea;
use picea::*;

#[test]
fn chain_call_order() {
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    enum State {
        PRE,
        WORKING,
        ENDING,
        DONE,
    }

    struct N {
        above: u32,
        below: u32,
        state: State,
    }

    impl Node<(), Rc<Cell<(u32, u32)>>> for N {
        type Output = Rc<Cell<(u32, u32)>>;
        type Event = ();

        fn update(&mut self, ctx: &mut Context<(), Rc<Cell<(u32, u32)>>, Self>) -> Rc<Cell<(u32, u32)>> {
            // First call made
            assert_eq!(self.state, State::PRE);
            self.state = State::WORKING;

            // Enqueue event to parent
            ctx.send(());

            // Only ancestors have started
            let p = ctx.param();
            let (started, done) = p.get();
            assert_eq!(self.above, started);
            assert_eq!(0, done);
            // self has started
            p.set((started + 1, done));

            // Pass counter onward
            p.clone()
        }

        fn event(&mut self, ctx: &mut Context<(), Rc<Cell<(u32, u32)>>, Self>, _: ()) {
            // Only `update` has been called
            assert_eq!(self.state, State::WORKING);
            self.state = State::ENDING;

            // All nodes started, all decedents have completed
            let p = ctx.param();
            let (started, done) = p.get();
            assert_eq!(self.above + self.below + 1, started);
            assert_eq!(self.below, done);
        }

        fn post(&mut self, ctx: &mut Context<(), Rc<Cell<(u32, u32)>>, Self>) {
            if self.below == 0 {
                // No children, only `update` has been called
                assert_eq!(self.state, State::WORKING);
            } else {
                // Has children, `update` and `event` have been called
                assert_eq!(self.state, State::ENDING);
            }
            self.state = State::DONE;

            // All nodes started, only decedents have completed
            let p = ctx.param();
            let (started, done) = p.get();
            assert_eq!(self.above + self.below + 1, started);
            assert_eq!(self.below, done);
            // self has completed
            p.set((started, done + 1));
        }
    }

    // Build a simple chain
    let mut t: Tree<(), Rc<Cell<(u32, u32)>>> = Tree::new();
    t.build()
        .push(N { above: 0, below: 4, state: State::PRE })
        .push(N { above: 1, below: 3, state: State::PRE })
        .push(N { above: 2, below: 2, state: State::PRE })
        .push(N { above: 3, below: 1, state: State::PRE })
        .push(N { above: 4, below: 0, state: State::PRE });
    // Update with no nodes started or completed
    t.update(&Rc::new(Cell::new((0, 0))));
}

#[test]
fn chain_data_sequence() {
    struct N {
        above: u32,
        below: u32,
    }

    impl Node<u32, u32> for N {
        type Output = u32;
        type Event = u32;

        fn update(&mut self, ctx: &mut Context<u32, u32, Self>) -> u32 {
            if self.below == 0 { ctx.send(1) }
            ctx.param() + 1
        }

        fn event(&mut self, ctx: &mut Context<u32, u32, Self>, n: u32) {
            assert_eq!(n, self.below);
            ctx.send(n + 1)
        }

        fn post(&mut self, ctx: &mut Context<u32, u32, Self>) {
            assert_eq!(*ctx.param(), self.above);
        }
    }

    // Build a simple chain
    let mut t: Tree<u32, u32> = Tree::new();
    t.build()
        .push(N { above: 0, below: 4 })
        .push(N { above: 1, below: 3 })
        .push(N { above: 2, below: 2 })
        .push(N { above: 3, below: 1 })
        .push(N { above: 4, below: 0 });
    t.update(&0);
    assert_eq!(t.events, vec![5]);
}

#[test]
fn add_sibling() {
    struct N {
        val: u32,
    }

    impl Node<u32, ()> for N {
        type Output = ();
        type Event = u32;

        fn update(&mut self, ctx: &mut Context<u32, (), Self>) {
            ctx.siblings().push(N { val: self.val + 1 });
        }

        fn event(&mut self, _ctx: &mut Context<u32, (), Self>, _n: u32) { }

        fn post(&mut self, ctx: &mut Context<u32, (), Self>) {
            ctx.send(self.val);
        }
    }

    // Build a simple chain
    let mut t: Tree<u32, ()> = Tree::new();
    t.build().push(N { val: 0 });
    t.update(&());
    assert_eq!(t.events, vec![0]);
    t.events.clear();
    t.update(&());
    assert_eq!(t.events, vec![0, 1]);
}
