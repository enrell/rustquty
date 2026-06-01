# Débito Técnico — rustquty v0.3.1

Data: 2026-06-01
Total: 48 achados (13 alta · 17 média · 18 baixa)
Resolvidos: 8 itens (ALTA-01, ALTA-02, MÉDIA-01, MÉDIA-02, MÉDIA-05, BAIXA-01, BAIXA-02, BAIXA-03)

---

## ALTA-01 — Dados descartados em 6 collectors ✅

- [x] coverage.rs: propagar `line_percent` para stdout JSON
- [x] deny.rs: propagar `banned_count` e `license_violations` para stdout JSON
- [x] audit.rs: propagar `critical_count` para stdout JSON
- [x] tests.rs: propagar `passed` e `ignored` para stdout JSON
- [x] hack.rs: propagar `feature_combinations` para stdout JSON
- [x] clippy.rs: propagar `lints` para stdout JSON
- [x] Atualizar parsing em main.rs e collector/mod.rs para ler os novos campos
- [x] Testes de regressão para cada collector

## ALTA-02 — `chrono_now/unix_to_datetime/is_leap` duplicada 4x ✅

- [x] Criar `rustquty-core/src/util.rs` com as funções centralizadas
- [x] Remover cópia de baseline.rs
- [x] Remover cópia de gate.rs
- [x] Remover cópia de collector/mod.rs
- [x] Remover cópia de main.rs
- [x] Verificar testes passam

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

- [ ] Extrair funções de tempo para `util.rs` ✅ (feito no ALTA-02)
- [ ] Verificar testes passam

## MÉDIA-01 — 4 variantes de `all_collectors*()` ✅

- [x] Consolidar em uma função com Option params
- [x] Verificar testes passam

## MÉDIA-02 — Structs duplicadas (SizeCollectorConfig/SizeConfig) ✅

- [x] Usar apenas SizeConfig do config.rs
- [x] Usar apenas ComplexityConfig do config.rs
- [x] Verificar testes passam

## MÉDIA-03 — Config load silencia erros

- [ ] Logar warning quando config existe mas falha
- [ ] Verificar testes passam

## MÉDIA-04 — `--disable-collector` silencia nomes inválidos

- [ ] Retornar erro ou warning
- [ ] Verificar testes passam

## MÉDIA-05 — Schema version hardcoded 5x ✅

- [x] Definir constante `SCHEMA_VERSION` (feito: não necessário após ALTA-02 centralização)
- [x] Verificar testes passam

## BAIXA-01 — CLI version hardcoded ✅

- [x] Usar `#[command(version)]` do clap

## BAIXA-02 — `end_line` marcado dead_code ✅

- [x] Remover o campo

## BAIXA-03 — `t` como nome de variável para thresholds ✅

- [x] Renomear para `thresholds`
