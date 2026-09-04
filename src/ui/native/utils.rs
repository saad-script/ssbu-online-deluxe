#![allow(dead_code)]

use skyline::nn::ui2d::{Pane, PaneFlag, TextBox};

#[skyline::from_offset(0x37A28A0)]
pub fn set_text_string(pane: *mut Pane, string: *const u8);

#[skyline::from_offset(0x59970)]
pub fn find_pane_by_name_recursive(pane: *const Pane, s: *const u8) -> *mut Pane;

pub trait PaneExt {
    fn print_info(&self);
    fn print_tree(&self);
    fn is_visible(&self) -> bool;
    fn influenced_alpha(&self) -> bool;
    fn set_influenced_alpha(&mut self, influenced_alpha: bool);
    fn next(&self) -> Option<&mut Pane>;
    fn prev(&self) -> Option<&mut Pane>;
    fn parent(&self) -> Option<&mut Pane>;
    fn children(&self) -> Option<&mut Pane>;
    fn traverse_upward(&self, steps: usize) -> Option<&mut Pane>;
    fn traverse_downward(&self, steps: usize) -> Option<&mut Pane>;
    fn traverse_forward(&self, steps: usize) -> Option<&mut Pane>;
    fn traverse_backward(&self, steps: usize) -> Option<&mut Pane>;
    fn find_child(&self, name: &str, recursive: bool) -> Option<&mut Pane>;
}

impl PaneExt for Pane {
    fn print_info(&self) {
        println!(
            "{}: Visible({}), BasePos({}), Pos({},{},{}), Alpha({},{},{})",
            self.get_name(),
            self.is_visible(),
            self.base_position,
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.alpha,
            self.global_alpha,
            self.influenced_alpha(),
        );
    }

    // dont use recursive impl, since overloading the stack will cause a crash
    fn print_tree(&self) {
        let mut stack = vec![(self, 0)];

        while let Some((node, depth)) = stack.pop() {
            for _ in 0..depth {
                print!("  ");
            }
            node.print_info();

            let mut children = Vec::new();
            let mut current = node.children();

            while let Some(child) = current {
                children.push(&*child);
                current = child.next();
            }

            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }

    fn is_visible(&self) -> bool {
        (self.flags & (1 << PaneFlag::Visible as u8)) != 0
    }

    fn influenced_alpha(&self) -> bool {
        (self.flags & (1 << PaneFlag::InfluencedAlpha as u8)) != 0
    }

    fn set_influenced_alpha(&mut self, influenced_alpha: bool) {
        match influenced_alpha {
            true => self.flags |= 1 << PaneFlag::InfluencedAlpha as u8,
            false => self.flags &= !(1 << PaneFlag::InfluencedAlpha as u8),
        }
    }

    fn next(&self) -> Option<&mut Pane> {
        unsafe {
            let node = self.link.next;
            let pane = ((node as *mut u64).sub(1)) as *mut Pane;
            match pane.is_null()
                || ((*pane).children_list.next.is_null() && (*pane).children_list.prev.is_null())
            {
                true => None,
                false => Some(&mut *pane),
            }
        }
    }

    fn prev(&self) -> Option<&mut Pane> {
        unsafe {
            let node = self.link.prev;
            let pane = ((node as *mut u64).sub(1)) as *mut Pane;
            match pane.is_null()
                || ((*pane).children_list.next.is_null() && (*pane).children_list.prev.is_null())
            {
                true => None,
                false => Some(&mut *pane),
            }
        }
    }

    fn parent(&self) -> Option<&mut Pane> {
        unsafe {
            let p = self.parent;
            match p.is_null() {
                true => None,
                false => Some(&mut *p),
            }
        }
    }

    fn children(&self) -> Option<&mut Pane> {
        unsafe {
            let node = self.children_list.next;
            let pane = ((node as *mut u64).sub(1)) as *mut Pane;
            match pane.is_null()
                || ((*pane).children_list.next.is_null() && (*pane).children_list.prev.is_null())
            {
                true => None,
                false => Some(&mut *pane),
            }
        }
    }

    fn traverse_upward(&self, steps: usize) -> Option<&mut Pane> {
        let mut i = 0;
        let mut current = self.parent();
        while let Some(p) = current {
            i += 1;
            if i == steps {
                return Some(p);
            }
            current = p.parent();
        }
        return None;
    }

    fn traverse_downward(&self, steps: usize) -> Option<&mut Pane> {
        let mut i = 0;
        let mut current = self.children();
        while let Some(p) = current {
            i += 1;
            if i == steps {
                return Some(p);
            }
            current = p.children();
        }
        return None;
    }

    fn traverse_forward(&self, steps: usize) -> Option<&mut Pane> {
        let mut i = 0;
        let mut current = self.next();
        while let Some(p) = current {
            i += 1;
            if i == steps {
                return Some(p);
            }
            current = p.next();
        }
        return None;
    }

    fn traverse_backward(&self, steps: usize) -> Option<&mut Pane> {
        let mut i = 0;
        let mut current = self.prev();
        while let Some(p) = current {
            i += 1;
            if i == steps {
                return Some(p);
            }
            current = p.prev();
        }
        return None;
    }

    fn find_child(&self, name: &str, recursive: bool) -> Option<&mut Pane> {
        if recursive {
            let child = unsafe {
                find_pane_by_name_recursive(self as *const Pane, format!("{}\0", name).as_ptr())
            };
            match child.is_null() {
                true => return None,
                false => return unsafe { Some(&mut *child) },
            }
        }

        let mut current = self.children();
        while let Some(p) = current {
            if p.get_name() == name {
                return Some(p);
            }
            current = p.next();
        }
        return None;
    }
}

pub trait TextBoxExt {
    fn set_text_string(&mut self, text: &str);
}

impl TextBoxExt for TextBox {
    fn set_text_string(&mut self, text: &str) {
        unsafe {
            set_text_string(
                self as *mut TextBox as *mut Pane,
                format!("{}\0", text).as_ptr(),
            );
        }
    }
}
