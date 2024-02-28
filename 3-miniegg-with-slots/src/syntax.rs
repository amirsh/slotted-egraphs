use crate::*;

///// parse

pub fn parse(s: &str) -> RecExpr {
    let (ast, s) = parse_ast(s);
    assert!(s.is_empty());

    let mut re = RecExpr::new();
    let (_, namemap) = translate(ast, &mut re);
    assert!(namemap.is_empty(), "Free variables are not allowed in parsed terms!");

    re
}

// adds the ENode corresponding to `ast` to `re`, and returns its `AppliedId`.
// each free variable in `ast` corresponds to a Slot in the returned HashMap.
// for (a: AppliedId, m: HashMap<..>) = translate(..); we require a.slots() == m.values();
fn translate(ast: Ast, re: &mut RecExpr) -> (AppliedId, HashMap<String, Slot>) {
    let mut out_id: AppliedId;
    let mut out_name_map;

    match ast {
        Ast::Lam(x, b) => {
            let (b, mut name_map) = translate(*b, re);

            // The slot in the ENode::Lam(..) that we will create.
            let lam_slot = Slot::fresh();

            let mut slotmap = SlotMap::new();
            for s in b.slots() {
                slotmap.insert(s, Slot::fresh());
            }

            match name_map.remove(&x) {
                Some(x_slot) => {
                    slotmap.insert(x_slot, lam_slot);
                },
                None => {},
            }

            let enode = ENode::Lam(lam_slot, b.apply_slotmap(&slotmap));
            out_id = re.push(enode);
            out_name_map = name_map;
        },
        Ast::App(l, r) => todo!(),
        Ast::Var(x) => {
            let s = Slot::fresh();
            out_id = re.push(ENode::Var(s));
            out_name_map = HashMap::new();
            out_name_map.insert(x, s);
        },
    };

    let slots: HashSet<_> = out_name_map.values().copied().collect();
    assert_eq!(out_id.slots(), slots);

    (out_id, out_name_map)
}

///// to_string

fn to_ast(re: &[ENode], name_map: HashMap<Slot, String>, namegen: &mut impl FnMut() -> String) -> Ast {
    let n = re.last().unwrap();
    match n {
        ENode::Lam(x, b) => {
            let xname = namegen();
            let mut sub_name_map = name_map.clone();
            sub_name_map.insert(*x, xname.clone());
            sub_name_map = sub_name_map.into_iter().map(|(x, y)| (b.m.inverse()[x], y)).collect();

            let b = to_ast(&re[0..b.id.0+1], sub_name_map, namegen);

            Ast::Lam(xname, Box::new(b))
        },
        ENode::App(l, r) => {
            let l = to_ast(&re[0..l.id.0+1], name_map.clone(), namegen);
            let r = to_ast(&re[0..r.id.0+1], name_map, namegen);

            Ast::App(Box::new(l), Box::new(r))
        },
        ENode::Var(x) => {
            let name = name_map[&x].clone();
            Ast::Var(name)
        },
    }
}

pub fn to_string(re: RecExpr) -> String {
    let mut name_id = 0;
    let mut namegen = || {
        let name = format!("x{name_id}");
        name_id += 1;

        name
    };

    let ast = to_ast(&re.node_dag, Default::default(), &mut namegen);
    ast_to_string(ast)
}

#[test]
fn test_parse_roundtrip() {
    let s1 = "(app (lam x0 x0) (lam x1 x1))";
    let (p, m) = parse(s1);
    let s2 = to_string(p, m);
    assert_eq!(s1, s2);
}
