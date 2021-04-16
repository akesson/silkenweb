use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    mem,
    rc::Rc,
};

use super::queue_effect;

#[derive(Clone, Default)]
pub struct Memo(Rc<RefCell<MemoData>>);

impl Memo {
    fn memo_map<'a, Key: 'static, Value: 'static>(
        any_map: &'a mut AnyMap,
    ) -> &'a mut HashMap<Key, Value> {
        let type_key = (TypeId::of::<Key>(), TypeId::of::<Value>());
        any_map
            .entry(type_key)
            .or_insert_with(|| Box::new(HashMap::<Key, Value>::new()))
            .downcast_mut()
            .unwrap()
    }

    pub fn cache<Key, Value, ValueFn>(&self, key: Key, value_fn: ValueFn) -> Value
    where
        Key: 'static + Eq + Hash,
        Value: 'static + Clone,
        ValueFn: FnOnce() -> Value,
    {
        let mut memo = self.0.borrow_mut();

        if memo.next_memoized.is_empty() {
            let memo_data = Rc::downgrade(&self.0);

            queue_effect(move || {
                if let Some(memo) = memo_data.upgrade() {
                    let mut memo = memo.borrow_mut();
                    memo.current_memoized = mem::take(&mut memo.next_memoized);
                }
            });
        }

        let current_memos = Self::memo_map::<Key, Value>(&mut memo.current_memoized);
        let value = current_memos.remove(&key).unwrap_or_else(value_fn);

        let next_memos = Self::memo_map::<Key, Value>(&mut memo.next_memoized);
        next_memos.insert(key, value.clone());
        value
    }
}

type AnyMap = HashMap<(TypeId, TypeId), Box<dyn Any>>;

#[derive(Default)]
struct MemoData {
    current_memoized: AnyMap,
    next_memoized: AnyMap,
}
