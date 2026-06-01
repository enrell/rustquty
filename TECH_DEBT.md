# Débito Técnico — rustquty v0.3.1

Data: 2026-06-01
Total: 48 achados (13 alta · 17 média · 18 baixa)

---

## ALTA-01 — Dados descartados em 6 collectors

Os collectors `coverage`, `deny`, `audit`, `clippy`, `tests`, `hack` fazem parsing de dados
mas descartam os valores com `_`. O MetricsSummary fica com zeros.

- [ ] coverage.rs: propagar `line_percent` para stdout JSON
- [ ] deny.rs: propagar `banned_count` e `license_violations` para stdout JSON
- [ ] audit.rs: propagar `critical_count` para stdout JSON
- [ ] tests.rs: propagar `passed` e `ignored` para stdout JSON
- [ ] hack.rs: propagar `feature_combinations` para stdout JSON
- [ ] clippy.rs: propagar `lints` para stdout JSON
- [ ] Atualizar parsing em main.rs e collector/mod.rs para ler os novos campos
- [ ] Testes de regressão para cada collector

## ALTA-02 — `chrono_now/unix_to_datetime/is_leap` duplicada 4x

Funções idênticas em baseline.rs, gate.rs, collector/mod.rs, main.rs (~240 linhas).

- [ ] Criar `rustquty-core/src/util.rs` com as funções centralizadas
- [ ] Remover cópia de baseline.rs
- [ ] Remover cópia de gate.rs
- [ ] Remover cópia de collector/mod.rs
- [ ] Remover cópia de main.rs
- [ ] Verificar testes passam

## ALTA-03 — `run_collectors()` duplicada entre core e main.rs

~460 linhas de lógica quase idêntica.

- [ ] Mover lógica de montagem do MetricsSummary para o core
- [ ] Simplificar main.rs para chamar o core
- [ ] Verificar testes passam

## ALTA-04 — `Gate::run()` com 274 linhas

- [ ] Extrair cada bloco de collector em função separada
- [ ] Verificar testes passam

## ALTA-05 — `main.rs` com lógica de negócio

- [ ] Mover `detect_rust_edition` e `parse_edition_from_content` para core
- [ ] Mover `is_collector_enabled` para core
- [ ] Verificar testes passam

## ALTA-06 — `collector/mod.rs` viola SRP

- [ ] Extrair funções de tempo para `util.rs`
- [ ] Verificar testes passam

## MÉDIA-01 — 4 variantes de `all_collectors*()`

- [ ] Consolidar em uma função com Option params
- [ ] Verificar testes passam

## MÉDIA-02 — Structs duplicadas (SizeCollectorConfig/SizeConfig)

- [ ] Usar apenas SizeConfig do config.rs
- [ ] Usar apenas ComplexityConfig do config.rs
- [ ] Verificar testes passam

## MÉDIA-03 — Config load silencia erros

- [ ] Logar warning quando config existe mas falha
- [ ] Verificar testes passam

## MÉDIA-04 — `--disable-collector` silencia nomes inválidos

- [ ] Retornar erro ou warning
- [ ] Verificar testes passam

## MÉDIA-05 — Schema version hardcoded 5x

- [ ] Definir constante `SCHEMA_VERSION`
- [ ] Verificar testes passam

## BAIXA-01 — CLI version hardcoded

- [ ] Usar `#[command(version)]` do clap

## BAIXA-02 — `end_line` marcado dead_code

- [ ] Remover ou usar o campo

## BAIXA-03 — `t` como nome de variável para thresholds

- [ ] Renomear para `thresholds`
