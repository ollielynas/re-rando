
use std::collections::{HashMap, HashSet};
use rand::prelude::SliceRandom;
// use rand::Rng;
use sycamore::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use median::stack::Filter;
use numfmt::*;
use turborand::prelude::*;
// extern crate console_error_panic_hook;
use wasm_timer;
use std::panic;

#[derive(Clone)]
struct Data {
    group: u8,
    value: f32,
}
#[derive(Clone)]
struct Table {
    data: Box<Vec<Data>>,
    group_1_ids: HashSet<u8>,
    group_2_ids: HashSet<u8>,
    group1: Vec<f32>,
    group2: Vec<f32>,
    mean_diff: f32,
    median_diff: f32,
    total_randomization: i32,
    mean_total: i32,
    median_total: i32,
    median_points: (usize, usize),
    rng: Rng,
    per_sec: f32,
    sum_total: f32,
    key: Vec<u8>,
    value: Vec<f32>,
    group1_total: f32,
    counters: (usize, usize),
    meds: (f32,f32),
}

impl Table {
    fn new(keys: Vec<String>, values: Vec<String>) -> Self {

        let mut new =  Self {
            data: Box::default(),
            group_1_ids: HashSet::new(),
            group_2_ids: HashSet::new(),
            group1: Vec::new(),
            group2: Vec::new(),
            mean_diff: 0.0,
            median_diff: 0.0,
            total_randomization: 0,
            mean_total: 0,
            median_total: 0,
            rng: Rng::with_seed(Default::default()),
            per_sec: 0.0,
            median_points: (0,0),
            sum_total: 0.0,
            key: Vec::new(),
            value: Vec::new(),
            group1_total: 0.0,
            counters: (0,0),
            meds: (0.0,0.0),
            
        };
        if keys.len() != values.len() || keys.len() == 0 || values.len() == 0 {
            return new
        }

        let mut key_list = HashMap::new();
        let binding = keys.iter().collect::<HashSet<_>>();
        for i in binding.iter().enumerate() {
            key_list.insert(i.1, i.0);
        }
        
        for (i, key) in keys.iter().enumerate() {
            let k = key.clone();
            if let Ok(val) = values[i].parse::<f32>() {
            new.data.push(Data {
                group: *key_list.get(&&k).unwrap() as u8,
                value: val,
            });
            new.sum_total += val;
        }
        }

        new.data.sort_by(|a,b|a.value.partial_cmp(&b.value).unwrap());

        new.group_1_ids = HashSet::from([0]);
        new.group_2_ids = (1..key_list.len() as u8).collect::<HashSet<u8>>();

        for i in new.data.as_mut() {
            if !new.group_1_ids.contains(&i.group) {
                i.group = 1;
            }
        }
        
        new.update_groups();

        // new.group1.sort_by(|a,b|a.partial_cmp(b).unwrap());
        // new.group2.sort_by(|a,b|a.partial_cmp(b).unwrap());

    
        if new.group1.len() == 0 {
            return new;
        }
        if new.group2.len() == 0 {
            return new;
        }

        let mut key = vec![];
        let mut value = vec![];

        for v in new.data.iter() {
            key.push(v.group);
            value.push(v.value);
        }

        new.key = key;
        new.value = value;

        new.median_points = (new.group1.len()/2, new.group2.len()/2);

        new.mean_diff = (new.group1.iter().sum::<f32>() / new.group1.len() as f32 - new.group2.iter().sum::<f32>() / new.group2.len() as f32).abs();
        new.median_diff = (new.group1[new.median_points.0] - new.group2[new.median_points.1]).abs();
        return new
    }

    fn update_groups(&mut self) {
        let mut group1 = Vec::new();
        let mut group2 = Vec::new();
        for data in self.data.iter() {
            if self.group_1_ids.contains(&data.group) {
                group1.push(data.value);
            } else {
                group2.push(data.value);
            }
        
        }
        self.group1 = group1;
        self.group2 = group2;
    }

    #[inline]
    fn re_randomize(&mut self) {
        self.rng.shuffle(&mut self.key);
        // self.key.shuffle(&mut self.rng);
        self.group1_total = 0.0;
        self.counters = (0,0);
        self.meds = (0.0,0.0);

        // optimization task 2
        for i in 0..self.key.len() {
            assert!(i < self.value.len());
            assert!(i < self.key.len());
            if self.key[i as usize] == 1 {
                if self.counters.0 == self.median_points.0 {
                    self.meds.0 = self.value[i];
                }
                self.counters.0 += 1;
                self.group1_total += self.value[i];
            } else if self.meds.1 == 0.0 {
                if self.counters.1 == self.median_points.1 {
                    self.meds.1 = self.value[i];
                }
                self.counters.1 += 1;
            }
        }

        if (self.group1_total / self.group1.len() as f32 - (self.sum_total - self.group1_total) / self.group2.len() as f32).abs() > self.mean_diff {
            self.mean_total += 1;
        }
        if (self.meds.0-self.meds.1).abs() > self.median_diff {
            self.median_total += 1;
        }
        self.total_randomization += 1;

    }

}

fn main() {
    // panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut fmt_per_sec = Formatter::default();
    
    


    sycamore::render(|cx| {        
        let table = create_signal(cx,Table::new(vec![], vec![]));
        let saves = match (<LocalStorage as Storage>::get("input1"), <LocalStorage as Storage>::get("input2")) {
            (Ok(a),Ok(b)) => (a,b),
            _ => (String::new(), String::new())
        };
        let input1 = create_signal(cx, saves.0);
        let input2 = create_signal(cx, saves.1);
        

        let google_docs = create_signal(cx, String::new());

        create_effect(cx, || {

            let text = (*google_docs.get()).clone();
            if text == String::new() {
                return;
            }
            let mut in_1 = String::new();
            let mut in_2 = String::new();
            for i in text.split("\n") {
                let mut j = i.split("\t");
                let k = j.next();
                let l = j.next();
                if let (Some(a), Some(b)) = (k,l) {
                    in_1.push_str(a);
                    in_1.push_str("\n");
                    in_2.push_str(b);
                    in_2.push_str("\n");
                }
            }
            input1.set(in_1);
            input2.set(in_2);
            
            let _ = <LocalStorage as Storage>::set("input1", (*input1.get_untracked()).clone());
            let _ = <LocalStorage as Storage>::set("input2", (*input2.get_untracked()).clone());
        });

        let re_randomize_count_string = create_signal(cx, String::new());
        let re_randomize_count = create_memo(cx, || {
            re_randomize_count_string.get().parse::<i32>().unwrap_or(10000)
        });



        create_effect(cx,  || {
            let in1 = input1.get().replace(",", " ").replace("\n", " ");
            let in2 = input2.get().replace(",", " ").replace("\n", " ");
            let keys = in1.split(" ").map(|s| s.to_string()).collect::<Vec<String>>();
            let values = in2.split(" ").map(|s| s.to_string()).collect::<Vec<String>>();
            table.set(Table::new(keys, values));
            let _ = <LocalStorage as Storage>::set("input1", (*input1.get()).clone());
            let _ = <LocalStorage as Storage>::set("input2", (*input2.get()).clone());
        });

        view! { cx,
        div (class = "vert") {
            p {("key value pairs")}
            div (class="inputs") {
                textarea (type="text", placeholder="group key", oninput="this.style.height = \"\";this.style.height = this.scrollHeight + \"px\"",  bind:value=input1)
                textarea (type="text", placeholder="value", oninput="this.style.height = \"\";this.style.height = this.scrollHeight + \"px\"", bind:value=input2)
            }
            
        p {("or paste from google docs")}
        textarea (type="text", placeholder="paste from google docs", bind:value=google_docs)
    }
        div (class="") {
            div (class="group") {
            p {(format!("total data: {}", table.get().data.len()))}
            p {(format!("group 1: {}", table.get().group1.len()))}
            p {(format!("group 2: {}", table.get().group2.len()))}
            p {(format!("mean diff: {}", table.get().mean_diff))}
            p {(format!("median diff: {}", table.get().median_diff))}
            p {(format!("total randomization: {}", table.get().total_randomization))}
            }
            div (class="group") {
            p {(format!("mean total: {}", table.get().mean_total))}
            p {(format!("median total: {}", table.get().median_total))}
            }
            div (class="group") {
            p {(format!("mean p: {}", (table.get().mean_total as f32)/((table.get().total_randomization as f32).max(1.0))))}
            p {(format!("median p: {}", (table.get().median_total as f32)/((table.get().total_randomization as f32).max(1.0))))}
            }
            
            div (class="group") {

                p {((format!("calculate {} times", re_randomize_count.get())))}
                input (type="number", bind:value=re_randomize_count_string) {("re-randomize count")}
                button (on:click= |_|{
                    let now = wasm_timer::Instant::now();
                    let mut t: Table = (*table.get_untracked()).clone();
                    for _ in 0..*re_randomize_count.get_untracked() {
                        t.re_randomize();
                    }
                    let elapsed = now.elapsed();
                    let per_sec = *re_randomize_count.get_untracked() as f32 / (elapsed.as_secs_f32());
                    t.per_sec = per_sec;
                    table.set(t);
                }) {}
            }

            div (class="group") {
                p {
                
                (format!("per second: {}", fmt_per_sec.fmt2(table.get().per_sec as f64)))
            }
            }

        }
    }});
}