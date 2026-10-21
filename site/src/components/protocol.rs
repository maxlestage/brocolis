//! La page « Protocole v3 » (`#/protocole`) : le double cliquet de la
//! messagerie, tel que brocolis le parle — comparatif, simulateur du cliquet,
//! déroulé pas à pas, anatomie d'un jeton, modèle de menace, plateformes,
//! limites et questions. Les textes sont dans `proto_text`.

use active::html::*;
use active::{Element, Node, State, use_state};

use crate::components::footer::site_footer;
use crate::components::marquee::marquee;
use crate::components::skip_link::skip_link;
use crate::components::switchers::switchers;
use crate::i18n::FR;
use crate::markup::{Translated, rolling, words};
use crate::proto_text::{Proto, SIM_LINES};
use crate::route::Route;
use crate::settings::t;

/// La spécification, dans le dépôt de la messagerie.
pub const SPEC: &str = "https://github.com/maxlestage/messagerie-/blob/master/docs/SECURITE.md";

/// Les ancres du sommaire, dans l'ordre de `Proto::toc`.
const ANCHORS: [&str; 7] = [
    "v3-change",
    "v3-sim",
    "v3-steps",
    "v3-token",
    "v3-threats",
    "v3-where",
    "v3-faq",
];

const MARQUEE: &[&str] = &[
    "Double Ratchet",
    "X25519",
    "ML-KEM-768",
    "Ed25519",
    "HKDF-SHA256",
    "AES-256-GCM",
    "PQXDH",
    "[[BROCOLIS:v3:…]]",
];

fn p3() -> &'static Proto {
    t().proto
}

// ── Le simulateur ────────────────────────────────────────────────────────────

/// Qui écrit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    You,
    Alice,
}

/// Ce qui s'affiche dans le fil du simulateur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Un message : son auteur, l'époque du cliquet (un échange
    /// Diffie-Hellman), son rang dans la chaîne, et sa ligne de texte (le
    /// rang de ses messages à lui).
    Msg {
        from: Who,
        epoch: u32,
        n: u32,
        line: usize,
    },
    /// Le grand cliquet tourne : l'autre a répondu.
    Turn,
    /// Un espion copie l'état de **votre** ordinateur.
    Spy,
}

/// Ce que l'espion peut faire d'un message.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exposure {
    /// Parti avant la copie : sa clé était effacée.
    Past,
    /// Lisible avec l'état copié.
    Exposed,
    /// Parti après que le cliquet a tourné de votre côté : clé inconnue.
    Healed,
}

/// Un double cliquet en miniature, pour voir où passe la frontière : ce
/// n'est pas de la cryptographie, seulement sa comptabilité.
///
/// L'état copié contient votre clé de cliquet courante. L'espion lit donc la
/// chaîne en cours **et**, si c'est vous qui avez écrit en dernier, la
/// réponse d'Alice (elle est dérivée de votre clé de cliquet). Il perd tout
/// dès que vous tirez une clé de cliquet neuve, c'est-à-dire à votre
/// prochain envoi après une réponse : la session a guéri.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sim {
    pub events: Vec<Event>,
    epoch: u32,
    n: u32,
    last: Option<Who>,
    sent: usize,
    /// Messages envoyés par vous, par Alice.
    by: [usize; 2],
    /// L'époque copiée, et qui avait écrit en dernier à ce moment-là.
    spy: Option<(usize, u32, Who)>,
}

impl Sim {
    pub fn sent(&self) -> usize {
        self.sent
    }

    pub fn spied(&self) -> bool {
        self.spy.is_some()
    }

    /// Envoie la ligne suivante ; `false` quand il n'y en a plus.
    pub fn send(&mut self, from: Who) -> bool {
        if self.sent >= SIM_LINES {
            return false;
        }
        match self.last {
            Some(last) if last == from => {}
            Some(_) => {
                self.epoch += 1;
                self.n = 0;
                self.events.push(Event::Turn);
            }
            None => {}
        }
        self.last = Some(from);
        self.n += 1;
        let line = &mut self.by[from as usize];
        self.events.push(Event::Msg {
            from,
            epoch: self.epoch,
            n: self.n,
            line: *line,
        });
        *line += 1;
        self.sent += 1;
        true
    }

    /// L'espion copie l'état (une fois, après au moins un message).
    pub fn spy(&mut self) -> bool {
        let Some(last) = self.last else { return false };
        if self.spy.is_some() {
            return false;
        }
        self.spy = Some((self.events.len(), self.epoch, last));
        self.events.push(Event::Spy);
        true
    }

    /// Ce que l'espion fait du `i`-ième événement (un message).
    pub fn exposure(&self, i: usize) -> Option<Exposure> {
        let (at, epoch, last) = self.spy?;
        let Event::Msg { epoch: e, .. } = *self.events.get(i)? else {
            return None;
        };
        Some(if i < at {
            Exposure::Past
        } else if e == epoch || (last == Who::You && e == epoch + 1) {
            Exposure::Exposed
        } else {
            Exposure::Healed
        })
    }

    /// Messages lisibles par l'espion.
    pub fn exposed(&self) -> usize {
        (0..self.events.len())
            .filter(|&i| self.exposure(i) == Some(Exposure::Exposed))
            .count()
    }
}

fn simulator() -> Element {
    let sim: State<Sim> = use_state(Sim::default());
    let button = |label: fn(&'static Proto) -> &'static str, class: &'static str| {
        button()
            .attr("type", "button")
            .class("btn sim-btn")
            .class(class)
            .child(rolling(move |s| label(s.proto)))
    };
    let log = div()
        .class("sim-log")
        .attr("role", "log")
        .attr("aria-live", "polite")
        .children_dyn(move || {
            sim.with(|s| {
                if s.events.is_empty() {
                    return vec![p().class("sim-empty").text(p3().sim_empty).into()];
                }
                s.events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| event_view(s, i, *e))
                    .collect()
            })
        });
    let score = p().class("sim-score").text_dyn(move || {
        sim.with(|s| {
            if s.spied() {
                p3().sim_score
                    .replace("{n}", &s.exposed().to_string())
                    .replace("{total}", &s.sent().to_string())
            } else {
                String::new()
            }
        })
    });
    div()
        .class("sim card")
        .attr("data-reveal", "card")
        .child(
            div()
                .class("sim-controls")
                .child(
                    button(|p| p.sim_send, "primary")
                        .bool_attr("disabled", move || sim.with(|s| s.sent() >= SIM_LINES))
                        .on_click(move |_| {
                            sim.update(|s| {
                                s.send(Who::You);
                            })
                        }),
                )
                .child(
                    button(|p| p.sim_reply, "")
                        .bool_attr("disabled", move || {
                            sim.with(|s| s.sent() == 0 || s.sent() >= SIM_LINES)
                        })
                        .on_click(move |_| {
                            sim.update(|s| {
                                s.send(Who::Alice);
                            })
                        }),
                )
                .child(
                    button(|p| p.sim_spy, "sim-spy")
                        .bool_attr("disabled", move || sim.with(|s| s.spied() || s.sent() == 0))
                        .on_click(move |_| {
                            sim.update(|s| {
                                s.spy();
                            })
                        }),
                )
                .child(
                    button(|p| p.sim_reset, "sim-reset")
                        .bool_attr("disabled", move || sim.with(|s| s.events.is_empty()))
                        .on_click(move |_| sim.set(Sim::default())),
                ),
        )
        .child(log)
        .child(score)
        .child(p().class("sim-note").raw(|t| t.proto.sim_note))
}

fn event_view(sim: &Sim, i: usize, e: Event) -> Node {
    let txt = p3();
    match e {
        Event::Turn => div()
            .class("sim-divider sim-turn")
            .text(format!("↻ {}", txt.sim_turn))
            .into(),
        Event::Spy => div()
            .class("sim-divider sim-spied")
            .text(format!("🕵️ {}", txt.sim_spied))
            .into(),
        Event::Msg {
            from,
            epoch,
            n,
            line,
        } => {
            let (side, who) = match from {
                Who::You => ("mine", txt.sim_you),
                Who::Alice => ("theirs", txt.sim_alice),
            };
            let badge = sim.exposure(i).map(|x| match x {
                Exposure::Exposed => span().class("sim-badge bad").text(txt.sim_exposed),
                Exposure::Past => span().class("sim-badge ok").text(txt.sim_safe_past),
                Exposure::Healed => span().class("sim-badge ok").text(txt.sim_safe_healed),
            });
            div()
                .class("sim-msg")
                .class(side)
                .child(span().class("sim-who").text(who))
                .child(p().class("sim-text").text(match from {
                    Who::You => txt.sim_you_texts[line],
                    Who::Alice => txt.sim_alice_texts[line],
                }))
                .child(span().class("sim-key").text(format!(
                    "🔑 {} K{}·{} — {}",
                    txt.sim_key,
                    epoch + 1,
                    n,
                    txt.sim_erased
                )))
                .child(badge)
                .into()
        }
    }
}

// ── La page ──────────────────────────────────────────────────────────────────

fn block(id: &'static str, title: fn(&'static Proto) -> &'static str) -> Element {
    section()
        .id(id)
        .class("v3-block")
        .attr("aria-labelledby", format!("{id}-h"))
        .child(
            h2().id(format!("{id}-h"))
                .attr("data-reveal", "words")
                .child(words(move |s| title(s.proto))),
        )
}

fn comparison() -> Element {
    let head = thead().child(
        tr().children((0..3).map(|i| th().attr("scope", "col").tr(move |s| s.proto.cmp_cols[i]))),
    );
    let rows = (0..FR.proto.cmp_rows.len()).map(|i| {
        tr().child(
            th().attr("scope", "row")
                .tr(move |s| s.proto.cmp_rows[i].label),
        )
        .child(td().class("old").tr(move |s| s.proto.cmp_rows[i].old))
        .child(td().class("new").tr(move |s| s.proto.cmp_rows[i].new))
    });
    block("v3-change", |p| p.cmp_h)
        .child(p().attr("data-reveal", "blur").raw(|t| t.proto.cmp_intro))
        .child(
            div()
                .class("table-wrap")
                .attr("tabindex", "0")
                .attr("data-reveal", "")
                .child(
                    table()
                        .class("v3-table cmp")
                        .child(head)
                        .child(tbody().children(rows)),
                ),
        )
}

fn steps() -> Element {
    let items = (0..FR.proto.steps.len()).map(|i| {
        li().class("card v3-step")
            .attr("data-reveal", "card")
            .child(h3().tr(move |s| s.proto.steps[i].title))
            .child(p().tr(move |s| s.proto.steps[i].body))
            .child(p().class("v3-detail").raw(move |s| s.proto.steps[i].detail))
    });
    block("v3-steps", |p| p.steps_h).child(ol().class("v3-steps").children(items))
}

fn token() -> Element {
    let cells = (0..FR.proto.token_fields.len()).map(|i| {
        let f = &FR.proto.token_fields[i];
        div()
            .class("tk-field")
            .class(if f.first_only { "first" } else { "" })
            .attr("data-reveal", "left")
            .child(
                span()
                    .class("tk-name")
                    .tr(move |s| s.proto.token_fields[i].name),
            )
            .child(
                span()
                    .class("tk-size")
                    .tr(move |s| s.proto.token_fields[i].size),
            )
            .child(
                span()
                    .class("tk-desc")
                    .tr(move |s| s.proto.token_fields[i].desc),
            )
    });
    block("v3-token", |p| p.token_h)
        .child(p().attr("data-reveal", "blur").raw(|t| t.proto.token_intro))
        .child(
            p().class("tk-legend")
                .child(span().class("tk-swatch").attr("aria-hidden", "true"))
                .tr(|t| t.proto.token_first),
        )
        .child(div().class("tk-row").children(cells))
        .child(
            p().class("note")
                .attr("data-reveal", "blur")
                .raw(|t| t.proto.token_note),
        )
}

fn threats() -> Element {
    let head = thead().child(tr().children((0..3).map(|i| {
        th().attr("scope", "col")
            .tr(move |s| s.proto.threat_cols[i])
    })));
    let rows = (0..FR.proto.threats.len()).map(|i| {
        tr().child(
            th().attr("scope", "row")
                .tr(move |s| s.proto.threats[i].who),
        )
        .child(td().tr(move |s| s.proto.threats[i].can))
        .child(td().class("new").tr(move |s| s.proto.threats[i].we))
    });
    block("v3-threats", |p| p.threat_h).child(
        div()
            .class("table-wrap")
            .attr("tabindex", "0")
            .attr("data-reveal", "")
            .child(
                table()
                    .class("v3-table")
                    .child(head)
                    .child(tbody().children(rows)),
            ),
    )
}

fn platforms() -> Element {
    let cards = (0..FR.proto.platforms.len()).map(|i| {
        article()
            .class("card")
            .attr("data-reveal", "card")
            .attr("data-tilt", "")
            .attr("data-spot", "")
            .child(h3().tr(move |s| s.proto.platforms[i].name))
            .child(p().tr(move |s| s.proto.platforms[i].status))
    });
    let labels = (0..FR.proto.labels.len()).map(|i| {
        li().attr("data-reveal", "left")
            .child(
                span()
                    .class("v3-label")
                    .tr(move |s| s.proto.labels[i].label),
            )
            .child(span().tr(move |s| s.proto.labels[i].new))
    });
    block("v3-where", |p| p.where_h)
        .child(p().attr("data-reveal", "blur").tr(|t| t.proto.where_intro))
        .child(div().class("grid").children(cards))
        .child(h3().class("v3-sub").tr(|t| t.proto.labels_h))
        .child(p().raw(|t| t.proto.labels_intro))
        .child(ul().class("v3-labels").children(labels))
        .child(h3().class("v3-sub").tr(|t| t.proto.limits_h))
        .child(ul().class("checklist v3-limits").children((0..5).map(|i| {
            li().attr("data-reveal", "left")
                .raw(move |s| s.proto.limits[i])
        })))
}

fn faq() -> Element {
    let items = (0..FR.proto.faqs.len()).map(|i| {
        details()
            .class("v3-faq")
            .attr("data-reveal", "")
            .child(summary().tr(move |s| s.proto.faqs[i].q))
            .child(p().raw(move |s| s.proto.faqs[i].a))
    });
    block("v3-faq", |p| p.faq_h).children(items)
}

fn spec() -> Element {
    section()
        .class("v3-spec card")
        .attr("aria-labelledby", "v3-spec-h")
        .attr("data-reveal", "card")
        .child(h2().id("v3-spec-h").tr(|t| t.proto.spec_h))
        .child(p().raw(|t| t.proto.spec_body))
        .child(
            a().class("btn primary")
                .attr("data-magnetic", "")
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
                .href(SPEC)
                .child(rolling(|t| t.proto.spec_cta)),
        )
}

pub fn protocol_page() -> Node {
    let toc = (0..ANCHORS.len()).map(|i| {
        li().child(
            a().href(format!("#{}", ANCHORS[i]))
                .on_click(move |ev| {
                    ev.prevent_default();
                    crate::dom::scroll_to_id(ANCHORS[i]);
                })
                .tr(move |s| s.proto.toc[i]),
        )
    });
    Node::from([
        skip_link(),
        nav()
            .class("navbar dl-navbar")
            .attr_tr("aria-label", |t| t.nav_menu)
            .child(
                div()
                    .class("container nav-inner")
                    .child(
                        a().class("nav-brand")
                            .href(Route::Home.href())
                            .child(
                                span()
                                    .class("nav-leaf")
                                    .attr("aria-hidden", "true")
                                    .text("🥦"),
                            )
                            .child(span().attr("data-scramble", "").text("Brocolis")),
                    )
                    .child(div().class("nav-switchers").child(switchers())),
            )
            .into(),
        main()
            .id("contenu")
            .class("container dl-main v3-main")
            .attr("tabindex", "-1")
            .child(
                a().class("dl-back")
                    .href(Route::Home.href())
                    .child(span().attr("aria-hidden", "true").text("← "))
                    .child(span().attr("data-scramble", "").tr(|t| t.proto.back)),
            )
            .child(
                p().class("v3-kicker")
                    .attr("data-scramble", "")
                    .tr(|t| t.proto.kicker),
            )
            .child(
                h1().id("v3-title")
                    .class("dl-title")
                    .attr("data-reveal", "words")
                    .child(words(|t| t.proto.title)),
            )
            .child(
                p().class("dl-intro v3-lead")
                    .attr("data-reveal", "blur")
                    .raw(|t| t.proto.lead),
            )
            .child(
                nav()
                    .class("v3-toc")
                    .attr_tr("aria-label", |t| t.proto.nav)
                    .child(ol().children(toc)),
            )
            .child(comparison())
            .child(
                block("v3-sim", |p| p.sim_h)
                    .child(p().attr("data-reveal", "blur").tr(|t| t.proto.sim_intro))
                    .child(simulator()),
            )
            .child(steps())
            .child(token())
            .child(threats())
            .child(platforms())
            .child(faq())
            .child(spec())
            .into(),
        marquee(false, false, Some(MARQUEE)),
        site_footer(false),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rien_de_ce_qui_est_parti_avant_la_copie_n_est_lisible() {
        let mut s = Sim::default();
        assert!(!s.spy(), "pas d'espion avant le premier message");
        s.send(Who::You); // K1·1
        s.send(Who::You); // K1·2
        assert!(s.spy());
        assert!(!s.spy(), "une seule copie");
        s.send(Who::You); // K1·3 : même chaîne, lisible
        s.send(Who::Alice); // K2·1 : dérivée de ma clé de cliquet copiée
        s.send(Who::You); // K3·1 : clé de cliquet neuve, guéri
        s.send(Who::Alice); // K4·1 : guéri
        let x: Vec<_> = (0..s.events.len()).filter_map(|i| s.exposure(i)).collect();
        assert_eq!(
            x,
            [
                Exposure::Past,
                Exposure::Past,
                Exposure::Exposed,
                Exposure::Exposed,
                Exposure::Healed,
                Exposure::Healed
            ]
        );
        assert_eq!(s.exposed(), 2);
        assert_eq!(s.sent(), 6);
    }

    #[test]
    fn copie_apres_une_reponse_alice_seule_est_exposee() {
        let mut s = Sim::default();
        s.send(Who::You);
        s.send(Who::Alice);
        s.spy();
        s.send(Who::Alice); // même chaîne qu'Alice : lisible
        s.send(Who::You); // je tourne le cliquet : guéri
        let x: Vec<_> = (0..s.events.len()).filter_map(|i| s.exposure(i)).collect();
        assert_eq!(
            x,
            [
                Exposure::Past,
                Exposure::Past,
                Exposure::Exposed,
                Exposure::Healed
            ]
        );
    }

    #[test]
    fn le_fil_s_arrete_a_la_derniere_ligne() {
        let mut s = Sim::default();
        for i in 0..SIM_LINES {
            assert!(s.send(if i % 2 == 0 { Who::You } else { Who::Alice }));
        }
        assert!(!s.send(Who::You));
        let turns = s.events.iter().filter(|e| **e == Event::Turn).count();
        assert_eq!(
            turns,
            SIM_LINES - 1,
            "le cliquet tourne à chaque changement d'auteur"
        );
    }

    #[test]
    fn les_tableaux_ont_la_meme_taille_dans_chaque_langue() {
        for p in [
            &crate::proto_text::FR,
            &crate::proto_text::EN,
            &crate::proto_text::ES,
        ] {
            assert!(p.sim_score.contains("{n}") && p.sim_score.contains("{total}"));
            assert_eq!(
                p.token_fields.iter().filter(|f| f.first_only).count(),
                5,
                "les champs du premier message"
            );
        }
    }
}
