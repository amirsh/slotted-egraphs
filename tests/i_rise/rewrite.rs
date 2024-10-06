use crate::*;
use crate::i_rise::build::*;

pub enum SubstMethod {
    Extraction,
    SmallStep,
    SmallStepUnoptimized,
}

pub fn rise_rules(subst_m: SubstMethod) -> Vec<Rewrite<RiseENode>> {
    let mut rewrites = Vec::new();

    rewrites.push(eta());
    rewrites.push(eta_expansion());

    rewrites.push(map_fusion());
    rewrites.push(map_fission());

    rewrites.push(remove_transpose_pair());
    rewrites.push(slide_before_map());
    rewrites.push(map_slide_before_transpose());
    rewrites.push(slide_before_map_map_f());
    rewrites.push(separate_dot_vh_simplified());
    rewrites.push(separate_dot_hv_simplified());

    match subst_m {
        SubstMethod::Extraction => {
            rewrites.push(beta_extr_direct());
        },
        SubstMethod::SmallStep => {
            rewrites.push(beta());
            rewrites.push(my_let_unused());
            rewrites.push(let_var_same());
            rewrites.push(let_app());
            rewrites.push(let_lam_diff());
        },
        SubstMethod::SmallStepUnoptimized => {
            rewrites.push(beta());
            rewrites.push(let_var_same());
            rewrites.push(let_var_diff());
            rewrites.push(let_app_unopt());
            rewrites.push(let_lam_diff_unopt());
            rewrites.push(let_const());
        },
    }

    rewrites
}

fn beta() -> Rewrite<RiseENode> {
    let pat = "(app (lam s1 ?body) ?e)";
    let outpat = "(let s1 ?e ?body)";

    Rewrite::new("beta", pat, outpat)
}

fn eta() -> Rewrite<RiseENode> {
    let pat = "(lam s1 (app ?f (var s1)))";
    let outpat = "?f";

    Rewrite::new_if("eta", pat, outpat, |subst| {
        !subst["f"].slots().contains(&Slot::new(1))
    })
}

fn eta_expansion() -> Rewrite<RiseENode> {
    let pat = "?f";
    let outpat = "(lam s1 (app ?f (var s1)))";

    Rewrite::new("eta-expansion", pat, outpat)
}

fn my_let_unused() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?t ?b)";
    let outpat = "?b";
    Rewrite::new_if("my-let-unused", pat, outpat, |subst| {
        !subst["b"].slots().contains(&Slot::new(1))
    })
}

fn let_var_same() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (var s1))";
    let outpat = "?e";
    Rewrite::new("let-var-same", pat, outpat)
}

fn let_var_diff() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (var s2))";
    let outpat = "(var s2)";
    Rewrite::new("let-var-diff", pat, outpat)
}

fn let_app() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (app ?a ?b))";
    let outpat = "(app (let s1 ?e ?a) (let s1 ?e ?b))";
    Rewrite::new_if("let-app", pat, outpat, |subst| {
        subst["a"].slots().contains(&Slot::new(1)) || subst["b"].slots().contains(&Slot::new(1))
    })
}

fn let_app_unopt() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (app ?a ?b))";
    let outpat = "(app (let s1 ?e ?a) (let s1 ?e ?b))";
    Rewrite::new("let-app-unopt", pat, outpat)
}

fn let_lam_diff() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (lam s2 ?body))";
    let outpat = "(lam s2 (let s1 ?e ?body))";
    Rewrite::new_if("let-lam-diff", pat, outpat, |subst| {
        subst["body"].slots().contains(&Slot::new(1))
    })
}

fn let_lam_diff_unopt() -> Rewrite<RiseENode> {
    let pat = "(let s1 ?e (lam s2 ?body))";
    let outpat = "(lam s2 (let s1 ?e ?body))";
    Rewrite::new("let-lam-diff-unopt", pat, outpat)
}

fn let_const() -> Rewrite<RiseENode> {
    // is the const-detection at the same time as the baseline? probably not relevant.
    let pat = Pattern::parse("(let s1 ?t ?c)").unwrap();

    let rt: RewriteT<RiseENode, ()> = RewriteT {
        searcher: Box::new(|_| ()),
        applier: Box::new(move |(), eg| {
            for subst in ematch_all(eg, &pat) {
                if eg.enodes_applied(&subst["c"]).iter().any(|n| matches!(n, RiseENode::Symbol(_) | RiseENode::Number(_))) {
                    let orig = pattern_subst(eg, &pat, &subst);
                    eg.union_justified(&orig, &subst["c"], Some("let-const".to_string()));
                }
            }
        }),
    };
    rt.into()
}

/////////////////////

fn map_fusion() -> Rewrite<RiseENode> {
    let mfu = "s0";
    let pat = "(app (app sym_map ?f) (app (app sym_map ?g) ?arg))";
    let outpat = &format!("(app (app sym_map (lam {mfu} (app ?f (app ?g (var {mfu}))))) ?arg)");
    Rewrite::new("map-fusion", pat, outpat)
}

fn map_fission() -> Rewrite<RiseENode> {
    let x = 0;
    let mfi = 1;

    let pat = &format!(
        "(app sym_map (lam s{x} (app ?f ?gx)))"
    );

    let outpat = &format!(
        "(lam s{mfi} (app (app sym_map ?f) (app (app sym_map (lam s{x} ?gx)) (var s{mfi}))))"
    );

    Rewrite::new_if("map-fission", pat, outpat, move |subst| {
        !subst["f"].slots().contains(&Slot::new(x))
    })
}

fn remove_transpose_pair() -> Rewrite<RiseENode> {
    let pat = "(app sym_transpose (app sym_transpose ?x))";
    let outpat = "?x";
    Rewrite::new("remove-transpose-pair", pat, outpat)
}

fn slide_before_map() -> Rewrite<RiseENode> {
    let pat = "(app (app (app sym_slide ?sz) ?sp) (app (app sym_map ?f) ?y))";
    let outpat = "(app (app sym_map (app sym_map ?f)) (app (app (app sym_slide ?sz) ?sp) ?y))";
    Rewrite::new("slide-before-map", pat, outpat)
}

fn map_slide_before_transpose() -> Rewrite<RiseENode> {
    let pat = "(app sym_transpose (app (app sym_map (app (app sym_slide ?sz) ?sp)) ?y))";
    let outpat = "(app (app sym_map sym_transpose) (app (app (app sym_slide ?sz) ?sp) (app sym_transpose ?y)))";
    Rewrite::new("map-slide-before-transpose", pat, outpat)
}

fn slide_before_map_map_f() -> Rewrite<RiseENode> {
    let pat = "(app (app sym_map (app sym_map ?f)) (app (app (app sym_slide ?sz) ?sp) ?y))";
    let outpat = "(app (app (app sym_slide ?sz) ?sp) (app (app sym_map ?f) ?y))";
    Rewrite::new("slide-before-map-map-f", pat, outpat)
}

fn separate_dot_vh_simplified() -> Rewrite<RiseENode> {
    let x = "s0";
    let sdvh = "s1";

    let pat = &format!(
        "(app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip (app sym_join sym_weights2d)) (app sym_join ?nbh))))
        ");
    let outpat = &format!(
        "(app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip sym_weightsH) (app (app sym_map (lam {sdvh} (app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip sym_weightsV) (var {sdvh})))))) (app sym_transpose ?nbh)))))
        ");
    Rewrite::new("separate-dot-vh-simplified", pat, outpat)
}

fn separate_dot_hv_simplified() -> Rewrite<RiseENode> {
    let x = "s0";
    let sdhv = "s1";

    let pat = &format!(
        "(app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip (app sym_join sym_weights2d)) (app sym_join ?nbh))))
        ");
    let outpat = &format!(
        "(app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip sym_weightsV) (app (app sym_map (lam {sdhv} (app (app (app sym_reduce sym_add) num_0) (app (app sym_map (lam {x} (app (app sym_mul (app sym_fst (var {x}))) (app sym_snd (var {x})))))
         (app (app sym_zip sym_weightsH) (var {sdhv})))))) ?nbh))))
        ");

    Rewrite::new("separate-dot-hv-simplified", pat, outpat)
}

// subst using extraction
fn beta_extr() -> Rewrite<RiseENode> {
    let pat = Pattern::parse("(app (lam s1 ?b) ?t)").unwrap();
    let s = Slot::new(1);

    let a = pat.clone();
    let a2 = pat.clone();

    let rt: RewriteT<RiseENode, Vec<(Subst, RecExpr<RiseENode>)>> = RewriteT {
        searcher: Box::new(move |eg| {
            let extractor = Extractor::<_, AstSize>::new(eg);

            let mut out: Vec<(Subst, RecExpr<RiseENode>)> = Vec::new();
            for subst in ematch_all(eg, &a) {
                let b = extractor.extract(subst["b"].clone(), eg);
                let t = extractor.extract(subst["t"].clone(), eg);
                let res = re_subst(s, b, &t);
                out.push((subst, res));
            }
            out
        }),
        applier: Box::new(move |substs, eg| {
            for (subst, res) in substs {
                let orig = pattern_subst(eg, &pat, &subst);
                let res = eg.add_expr(res);
                eg.union_justified(&orig, &res, Some("beta-expr".to_string()));
            }
        }),
    };
    rt.into()
}

// why is this faster than beta_extr?
// Probably because it can extract smaller terms after more rewrites?
fn beta_extr_direct() -> Rewrite<RiseENode> {
    let pat = Pattern::parse("(app (lam s1 ?b) ?t)").unwrap();
    let s = Slot::new(1);

    let a = pat.clone();
    let a2 = pat.clone();

    let rt: RewriteT<RiseENode, ()> = RewriteT {
        searcher: Box::new(|_| ()),
        applier: Box::new(move |(), eg| {
            let extractor = Extractor::<_, AstSize>::new(eg);

            let mut out: Vec<(Subst, RecExpr<RiseENode>)> = Vec::new();
            for subst in ematch_all(eg, &a) {
                let b = extractor.extract(subst["b"].clone(), eg);
                let t = extractor.extract(subst["t"].clone(), eg);
                let res = re_subst(s, b, &t);
                out.push((subst, res));
            }
            for (subst, res) in out {
                let orig = pattern_subst(eg, &pat, &subst);
                let res = eg.add_expr(res);
                eg.union_justified(&orig, &res, Some("betaoextr-direct".to_string()));
            }
        }),
    };
    rt.into()
}

fn re_subst(s: Slot, b: RecExpr<RiseENode>, t: &RecExpr<RiseENode>) -> RecExpr<RiseENode> {
    let new_node = match b.node {
        RiseENode::Var(s2) if s == s2 => return t.clone(),
        RiseENode::Lam(s2, _) if s == s2 => panic!("This shouldn't be possible!"),
        RiseENode::Let(..) => panic!("This shouldn't be here!"),
        old => old,
    };

    let mut children = Vec::new();
    for child in b.children {
        children.push(re_subst(s, child, t));
    }

    RecExpr {
        node: new_node,
        children,
    }
}
