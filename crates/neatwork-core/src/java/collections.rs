//! Java collections whose iteration order reaches the NeatWork output files.
//!
//! The Java program writes several sections by iterating a `Hashtable` or a
//! `Properties` object (which is backed by a `ConcurrentHashMap`). Their
//! order depends on the hash codes, the table size and the insertion
//! history. These types copy the JDK 25 algorithms
//! (`java.util.Hashtable`, `java.util.concurrent.ConcurrentHashMap`,
//! `java.util.Properties`) for a single thread, so the Rust program writes
//! the same lines in the same order.

/// `Object.hashCode()` of the Java key.
pub trait JavaHash {
    fn java_hash(&self) -> i32;
}

impl JavaHash for String {
    fn java_hash(&self) -> i32 {
        string_hash(self)
    }
}

impl JavaHash for i32 {
    fn java_hash(&self) -> i32 {
        *self
    }
}

/// `String.hashCode()`: s[0]*31^(n-1) + ... over UTF-16 code units.
pub fn string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

const MAX_ARRAY_SIZE: i64 = i32::MAX as i64 - 8;

// ---------------------------------------------------------------- Hashtable

#[derive(Clone, Debug)]
struct HtEntry<K, V> {
    hash: i32,
    key: K,
    value: V,
}

/// `java.util.Hashtable` with load factor 0.75. Each bucket is stored head
/// first.
#[derive(Clone, Debug)]
pub struct JHashtable<K, V> {
    table: Vec<Vec<HtEntry<K, V>>>,
    count: usize,
    threshold: i64,
}

impl<K: JavaHash + PartialEq + Clone, V: Clone> Default for JHashtable<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: JavaHash + PartialEq + Clone, V: Clone> JHashtable<K, V> {
    /// `new Hashtable()`: capacity 11.
    pub fn new() -> Self {
        Self::with_capacity(11)
    }

    /// `new Hashtable(initialCapacity, 0.75f)`.
    pub fn with_capacity(initial: usize) -> Self {
        let cap = initial.max(1);
        let mut table = Vec::with_capacity(cap);
        table.resize_with(cap, Vec::new);
        JHashtable {
            table,
            count: 0,
            threshold: threshold(cap),
        }
    }

    /// `new Hashtable(Map t)`: capacity max(2 * size, 11), then `putAll`.
    pub fn from_hashtable(t: &JHashtable<K, V>) -> Self {
        let mut h = Self::with_capacity((2 * t.len()).max(11));
        for (k, v) in t.entries() {
            h.put(k.clone(), v.clone());
        }
        h
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn index(&self, hash: i32) -> usize {
        ((hash & 0x7FFF_FFFF) as usize) % self.table.len()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = key.java_hash();
        self.table[self.index(hash)]
            .iter()
            .find(|e| e.hash == hash && e.key == *key)
            .map(|e| &e.value)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// `put`: replaces the value of an existing key in place, otherwise adds
    /// the entry at the head of its bucket (after a rehash if needed).
    pub fn put(&mut self, key: K, value: V) {
        let hash = key.java_hash();
        let idx = self.index(hash);
        if let Some(e) = self.table[idx]
            .iter_mut()
            .find(|e| e.hash == hash && e.key == key)
        {
            e.value = value;
            return;
        }
        if self.count as i64 >= self.threshold {
            self.rehash();
        }
        let idx = self.index(hash);
        self.table[idx].insert(0, HtEntry { hash, key, value });
        self.count += 1;
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = key.java_hash();
        let idx = self.index(hash);
        let pos = self.table[idx]
            .iter()
            .position(|e| e.hash == hash && e.key == *key)?;
        self.count -= 1;
        Some(self.table[idx].remove(pos).value)
    }

    /// `clear`: empties the buckets but keeps the capacity.
    pub fn clear(&mut self) {
        for b in &mut self.table {
            b.clear();
        }
        self.count = 0;
    }

    fn rehash(&mut self) {
        let old_cap = self.table.len();
        let new_cap = old_cap * 2 + 1;
        let old = std::mem::take(&mut self.table);
        self.table.resize_with(new_cap, Vec::new);
        self.threshold = threshold(new_cap);
        for bucket in old.into_iter().rev() {
            for e in bucket {
                let idx = self.index(e.hash);
                self.table[idx].insert(0, e);
            }
        }
    }

    /// Entries in the order of `keys()`, `elements()` and `entrySet()`:
    /// buckets from the last to the first, each bucket head first.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table
            .iter()
            .rev()
            .flat_map(|b| b.iter().map(|e| (&e.key, &e.value)))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries().map(|(_, v)| v)
    }
}

fn threshold(cap: usize) -> i64 {
    // (int) Math.min(initialCapacity * loadFactor, MAX_ARRAY_SIZE + 1),
    // computed in float as Java does.
    let t = (cap as f32 * 0.75f32) as f64;
    t.min((MAX_ARRAY_SIZE + 1) as f64) as i64
}

// -------------------------------------------------------- ConcurrentHashMap

const TREEIFY_THRESHOLD: usize = 8;
const MIN_TREEIFY_CAPACITY: usize = 64;
const MAXIMUM_CAPACITY: usize = 1 << 30;

#[derive(Clone, Debug)]
struct ChmNode<K, V> {
    hash: i32,
    key: K,
    value: V,
}

/// `java.util.concurrent.ConcurrentHashMap`, single thread. Bins keep
/// insertion order (new nodes go to the tail).
#[derive(Clone, Debug)]
pub struct JConcurrentHashMap<K, V> {
    table: Option<Vec<Vec<ChmNode<K, V>>>>,
    size_ctl: i64,
    count: usize,
}

fn spread(h: i32) -> i32 {
    (h ^ ((h as u32) >> 16) as i32) & 0x7FFF_FFFF
}

fn table_size_for(c: usize) -> usize {
    if c <= 1 {
        1
    } else {
        c.next_power_of_two().min(MAXIMUM_CAPACITY)
    }
}

impl<K: JavaHash + PartialEq + Clone, V: Clone> JConcurrentHashMap<K, V> {
    /// `new ConcurrentHashMap(initialCapacity)`.
    pub fn new(initial_capacity: usize) -> Self {
        let ic = initial_capacity.max(1);
        // (long)(1.0 + (long)initialCapacity / loadFactor), loadFactor 0.75f
        let size = (1.0 + f64::from(ic as f32 / 0.75f32)) as usize;
        let cap = if size >= MAXIMUM_CAPACITY {
            MAXIMUM_CAPACITY
        } else {
            table_size_for(size)
        };
        JConcurrentHashMap {
            table: None,
            size_ctl: cap as i64,
            count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let tab = self.table.as_ref()?;
        let hash = spread(key.java_hash());
        tab[(tab.len() - 1) & hash as usize]
            .iter()
            .find(|n| n.hash == hash && n.key == *key)
            .map(|n| &n.value)
    }

    fn init_table(&mut self) {
        let n = if self.size_ctl > 0 {
            self.size_ctl as usize
        } else {
            16
        };
        let mut t = Vec::with_capacity(n);
        t.resize_with(n, Vec::new);
        self.table = Some(t);
        self.size_ctl = (n - (n >> 2)) as i64;
    }

    /// `put` (`putVal(key, value, false)`).
    pub fn put(&mut self, key: K, value: V) {
        let hash = spread(key.java_hash());
        if self.table.is_none() {
            self.init_table();
        }
        let Some(tab) = self.table.as_mut() else {
            return;
        };
        let n = tab.len();
        let i = (n - 1) & hash as usize;
        let bin = &mut tab[i];
        let bin_count;
        if bin.is_empty() {
            bin.push(ChmNode { hash, key, value });
            bin_count = 0;
        } else {
            if let Some(pos) = bin.iter().position(|e| e.hash == hash && e.key == key) {
                bin[pos].value = value;
                if pos + 1 >= TREEIFY_THRESHOLD {
                    self.treeify_bin();
                }
                return;
            }
            bin_count = bin.len();
            bin.push(ChmNode { hash, key, value });
            if bin_count >= TREEIFY_THRESHOLD {
                self.treeify_bin();
            }
        }
        self.add_count(bin_count);
    }

    fn treeify_bin(&mut self) {
        // Tables below 64 bins grow instead of building a tree. NeatWork
        // property sets never reach a tree bin.
        if let Some(t) = &self.table {
            let n = t.len();
            if n < MIN_TREEIFY_CAPACITY {
                self.try_presize(n << 1);
            }
        }
    }

    fn add_count(&mut self, _check: usize) {
        self.count += 1;
        while let Some(t) = &self.table {
            let n = t.len();
            if (self.count as i64) < self.size_ctl || n >= MAXIMUM_CAPACITY {
                break;
            }
            self.transfer();
        }
    }

    fn try_presize(&mut self, size: usize) {
        let c = if size >= (MAXIMUM_CAPACITY >> 1) {
            MAXIMUM_CAPACITY
        } else {
            table_size_for(size + (size >> 1) + 1)
        };
        loop {
            let sc = self.size_ctl;
            match &self.table {
                None => {
                    let n = (sc as usize).max(c);
                    let mut t = Vec::with_capacity(n);
                    t.resize_with(n, Vec::new);
                    self.table = Some(t);
                    self.size_ctl = (n - (n >> 2)) as i64;
                }
                Some(t) => {
                    let n = t.len();
                    if (c as i64) <= sc || n >= MAXIMUM_CAPACITY {
                        break;
                    }
                    self.transfer();
                }
            }
        }
    }

    /// Double the table. Each bin splits into a low list and a high list;
    /// the trailing run keeps its order, the nodes before it are prepended.
    fn transfer(&mut self) {
        let Some(old) = self.table.take() else {
            return;
        };
        let n = old.len();
        let mut next: Vec<Vec<ChmNode<K, V>>> = Vec::with_capacity(n << 1);
        next.resize_with(n << 1, Vec::new);
        for (i, bin) in old.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }
            let bit = |h: i32| (h as usize) & n;
            let mut run_bit = bit(bin[0].hash);
            let mut last_run = 0;
            for (p, node) in bin.iter().enumerate().skip(1) {
                let b = bit(node.hash);
                if b != run_bit {
                    run_bit = b;
                    last_run = p;
                }
            }
            let mut ln: Vec<ChmNode<K, V>> = Vec::new();
            let mut hn: Vec<ChmNode<K, V>> = Vec::new();
            let mut nodes = bin;
            let tail = nodes.split_off(last_run);
            if run_bit == 0 {
                ln = tail;
            } else {
                hn = tail;
            }
            for node in nodes {
                if bit(node.hash) == 0 {
                    ln.insert(0, node);
                } else {
                    hn.insert(0, node);
                }
            }
            next[i] = ln;
            next[i + n] = hn;
        }
        self.table = Some(next);
        self.size_ctl = ((n << 1) - (n >> 1)) as i64;
    }

    /// `putAll(m)`: presize when the table exists, then put in `m`'s order.
    pub fn put_all<'a>(&mut self, entries: impl Iterator<Item = (&'a K, &'a V)>, m_size: usize)
    where
        K: 'a,
        V: 'a,
    {
        if self.table.is_some() {
            self.try_presize(self.count + m_size);
        }
        for (k, v) in entries {
            self.put(k.clone(), v.clone());
        }
    }

    /// Iteration order: bins from first to last, each bin in list order.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table
            .iter()
            .flat_map(|t| t.iter())
            .flat_map(|b| b.iter().map(|n| (&n.key, &n.value)))
    }
}

// --------------------------------------------------------------- Properties

/// `java.util.Properties` with its `defaults` chain. The defaults are a
/// snapshot: NeatWork never changes a defaults object after it hands it to
/// a child.
#[derive(Clone, Debug)]
pub struct JProperties {
    map: JConcurrentHashMap<String, String>,
    defaults: Option<Box<JProperties>>,
}

impl Default for JProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl JProperties {
    /// `new Properties()`.
    pub fn new() -> Self {
        JProperties {
            map: JConcurrentHashMap::new(8),
            defaults: None,
        }
    }

    /// `new Properties(defaults)`.
    pub fn with_defaults(defaults: &JProperties) -> Self {
        JProperties {
            map: JConcurrentHashMap::new(8),
            defaults: Some(Box::new(defaults.clone())),
        }
    }

    pub fn set_property(&mut self, key: &str, value: &str) {
        self.map.put(key.to_string(), value.to_string());
    }

    pub fn get_property(&self, key: &str) -> Option<&str> {
        match self.map.get(&key.to_string()) {
            Some(v) => Some(v.as_str()),
            None => self.defaults.as_ref().and_then(|d| d.get_property(key)),
        }
    }

    pub fn get_property_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get_property(key).unwrap_or(default)
    }

    /// `putAll(other)`: copies the entries of `other` itself, not its
    /// defaults.
    pub fn put_all(&mut self, other: &JProperties) {
        self.map.put_all(other.map.entries(), other.map.len());
    }

    /// Own entries (no defaults), in `entrySet()` order.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &String)> {
        self.map.entries()
    }

    /// `propertyNames()`: a fresh `Hashtable` filled with the defaults
    /// (recursively) then the own entries; its key order.
    pub fn property_names(&self) -> Vec<String> {
        let mut h: JHashtable<String, String> = JHashtable::new();
        self.enumerate(&mut h);
        h.keys().cloned().collect()
    }

    fn enumerate(&self, h: &mut JHashtable<String, String>) {
        if let Some(d) = &self.defaults {
            d.enumerate(h);
        }
        for (k, v) in self.map.entries() {
            h.put(k.clone(), v.clone());
        }
    }

    /// `load(Reader)` for the simple files NeatWork ships: comments,
    /// `key = value`, `key: value`, `key value`, backslash escapes and line
    /// continuations.
    pub fn load(&mut self, text: &str) {
        let mut lines = text.lines();
        while let Some(first) = lines.next() {
            let mut line = first.trim_start_matches([' ', '\t', '\u{c}']).to_string();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            while ends_with_odd_backslash(&line) {
                line.pop();
                match lines.next() {
                    Some(next) => line.push_str(next.trim_start_matches([' ', '\t', '\u{c}'])),
                    None => break,
                }
            }
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            let mut key = String::new();
            while i < chars.len() {
                let c = chars[i];
                if c == '\\' && i + 1 < chars.len() {
                    key.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '=' || c == ':' || c == ' ' || c == '\t' || c == '\u{c}' {
                    break;
                }
                key.push(c);
                i += 1;
            }
            while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\u{c}') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '=' || chars[i] == ':') {
                i += 1;
            }
            while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\u{c}') {
                i += 1;
            }
            let raw: String = chars[i..].iter().collect();
            self.set_property(&unescape(&key), &unescape(&raw));
        }
    }
}

fn ends_with_odd_backslash(s: &str) -> bool {
    s.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\u{c}'),
            Some('u') => {
                let hex: String = (0..4).filter_map(|_| it.next()).collect();
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_hash_like_java() {
        assert_eq!(string_hash(""), 0);
        assert_eq!(string_hash("a"), 97);
        assert_eq!(string_hash("ab"), 97 * 31 + 98);
    }

    #[test]
    fn hashtable_integer_order() {
        // new Hashtable<Integer,_>(); put 0..12 -> rehash to 23 at the 9th put
        let mut h: JHashtable<i32, i32> = JHashtable::new();
        for i in 0..12 {
            h.put(i, i);
        }
        let keys: Vec<i32> = h.keys().copied().collect();
        assert_eq!(keys, vec![11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    }
}
