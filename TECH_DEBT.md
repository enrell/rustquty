# Débito Técnico — rustquty v0.3.1

Data: 2026-06-01
Total: 48 achados (13 alta · 17 média · 18 baixa)
Resolvidos: 14 itens (todos)

---

## ALTA-01 — Dados descartados em 6 collectors ✅

## ALTA-02 — `chrono_now/unix_to_datetime/is_leap` duplicada 4x ✅

## ALTA-03 — `run_collectors()` duplicada entre core e main.rs ✅

- [x] Separar execute_collectors + assemble_results no core
- [x] main.rs delega para o core

## ALTA-04 — `Gate::run()` com 274 linhas ✅

- [x] Refatorado com macros check_pass! e check_status!
- [x] Reduzido de ~270 para ~120 linhas

## ALTA-05 — `main.rs` com lógica de negócio ✅

- [x] run_collectors reduzida de ~240 para ~25 linhas
- [x] Lógica de assembly movida para core

## ALTA-06 — `collector/mod.rs` viola SRP ✅

- [x] Funções de tempo extraídas para util.rs (feito no ALTA-02)
- [x] execute_collectors e assemble_results separados

## MÉDIA-01 — 4 variantes de `all_collectors*()` ✅

## MÉDIA-02 — Structs duplicadas (SizeCollectorConfig/SizeConfig) ✅

## MÉDIA-03 — Config load silencia erros ✅

- [x] Warning quando TOML existe mas falha ao carregar

## MÉDIA-04 — `--disable-collector` silencia nomes inválidos ✅

- [x] Warning quando nome de collector é inválido

## MÉDIA-05 — Schema version hardcoded 5x ✅

## BAIXA-01 — CLI version hardcoded ✅

## BAIXA-02 — `end_line` marcado dead_code ✅

## BAIXA-03 — `t` como nome de variável para thresholds ✅
