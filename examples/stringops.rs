extern crate picea;
use picea::*;

use std::collections::VecDeque;

type Tick = ();

#[derive(Clone)]
struct Timer {
    t: u32,
    event: TextEvent,
}

impl Node<TextEvent, Tick> for VecDeque<Timer> {
    type Output = ();
    type Event = ();

    fn update(
        &mut self,
        ctx: &mut Context<TextEvent, Tick, Self>,
    ) {
        if let Some(last) = self.front_mut() {
            if last.t > 0 {
                last.t -= 1;
                return;
            }
        }
        if let Some(t) = self.pop_front() { ctx.send(t.event); }
    }

    fn event(
        &mut self,
        _: &mut Context<TextEvent, Tick, Self>,
        _: (),
    ) { }

    fn post(
        &mut self,
        _: &mut Context<TextEvent, Tick, Self>,
    ) { }
}

#[derive(Clone)]
enum TextEvent {
    Move(f32),
    Duplicate(f32, VecDeque<Timer>),
    Append(String),
    Delete,
}

impl TextEvent {
    fn timer(self, t: u32) -> Timer {
        Timer { t: t, event: self }
    }
}

struct Text {
    pos: f32,
    val: String
}

impl Text {
    fn new(pos: f32, val: &str) -> Text {
        Text { pos: pos, val: val.to_owned() }
    }
}

impl Node<(f32, String), Tick> for Text {
    type Output = Tick;
    type Event = TextEvent;

    fn update(
        &mut self,
        ctx: &mut Context<(f32, String), Tick, Self>,
    ) -> Tick {
        ctx.send((self.pos, self.val.clone()));
        *ctx.param()
    }

    fn event(
        &mut self,
        ctx: &mut Context<(f32, String), Tick, Self>,
        event: TextEvent,
    ) {
        use self::TextEvent::*;
        match event {
            Move(v) => self.pos += v,
            Delete => ctx.kill(),
            Duplicate(pos, timers) => {
                ctx.siblings().push(Text {
                    pos: pos,
                    val: self.val.clone(),
                }).push(timers);
            },
            Append(ref s) => self.val += s,
        }
    }

    fn post(
        &mut self,
        _: &mut Context<(f32, String), Tick, Self>,
    ) { }
}

struct SortedCat {
    text: Vec<(f32, String)>,
}

impl SortedCat {
    fn new() -> SortedCat { SortedCat { text: Vec::new() }}
}

impl Node<String, Tick> for SortedCat {
    type Output = Tick;
    type Event = (f32, String);

    fn update(
        &mut self,
        ctx: &mut Context<String, Tick, Self>,
    ) -> Tick {
        *ctx.param()
    }

    fn event(
        &mut self,
        _: &mut Context<String, Tick, Self>,
        event: (f32, String),
    ) {
        self.text.push(event);
    }

    fn post(
        &mut self,
        ctx: &mut Context<String, Tick, Self>,
    ) {
        self.text.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ctx.send(self.text.iter().fold(String::new(), |a, b| a + " " + &b.1));
        self.text.clear();
    }
}

macro_rules! deque {
    [$($e:expr),*$(,)*] => {
        VecDeque::from(vec![$($e),*])
    };
}

fn main() {
    use self::TextEvent::*;

    let mut tree = Tree::new();
    { let mut root = tree.build();
        { let mut cat = root.push(SortedCat::new());
            cat.push(Text::new(0., "Hello")).push(deque![
                Move(2.).timer(0),
                Delete.timer(7),
            ]);
            cat.push(Text::new(1., "World")).push(deque![
                Move(2.).timer(1),
                Duplicate(-1., deque![
                    Append("est".to_owned()).timer(0),
                    Move(100.).timer(0),
                    Delete.timer(0),
                ]).timer(0),
            ]);
        }
        { let mut cat = root.push(SortedCat::new());
            cat.push(Text::new(0., "Foo")).push(deque![
                Move(2.).timer(0),
                Append("oo".to_owned()).timer(1),
            ]);
            cat.push(Text::new(1., "Bar")).push(deque![
                Move(2.).timer(1),
                Delete.timer(2),
            ]);
        }
    }
    
    for _ in 0 .. 10 {
        println!("=============");
        tree.update(&());
        for e in tree.events.drain(..) {
            println!("{}", e);
        }
    }
}
