# EXSUL — Техническое задание v2.0
## Режим «Бизнес: Цветы» — полная переработка

> **Для исполнителя:** Это production-приложение Tauri v2 + SvelteKit + Rust/SQLite.  
> Стек: Rust (backend, команды, БД), SvelteKit 5 (Svelte Runes), SQLite через rusqlite.  
> Версия приложения: **0.0.7**. Всё ниже — обязательные изменения.

---

## 0. Контекст и принципы

**Что делает программа:** учёт цветочного бизнеса (семья). Теплицы → упаковка → склад → заказы → аналитика.  
**Пользователь:** обычный человек, не технарь. Интерфейс должен быть понятен без инструкции.  
**Принцип:** минимальный сюрприз, максимальная ясность. Ничего не должно быть "магическим".

### Текущая структура навигации (flowers preset):
```
Дашборд → Теплица (/flowers) → Склад (/inventory) → Заказы (/orders) → Аналитика (/analytics) → Настройки → Синхронизация
```

### Целевая структура навигации (flowers preset):
```
1. Дашборд (/)
2. Теплица (/flowers)        ← уже есть, дорабатываем
3. Склад (/inventory)        ← переработка: упаковка + карточки товара
4. Заказы (/orders)          ← полная переработка + печать
5. Аналитика (/analytics)    ← только количество, без выручки в основном виде
6. Настройки (/settings)
7. Синхронизация (модал)
```

---

## 1. БАЗА ДАННЫХ — новые миграции

### Миграция 010: расширение flower_sorts и greenhouse_raw

```sql
-- Файл: src-tauri/migrations/010_greenhouse_warehouse.sql

-- Добавить поле photo_path к flower_sorts (уже может быть, проверить)
ALTER TABLE flower_sorts ADD COLUMN IF NOT EXISTS photo_path TEXT;
ALTER TABLE flower_sorts ADD COLUMN IF NOT EXISTS description TEXT;
ALTER TABLE flower_sorts ADD COLUMN IF NOT EXISTS total_harvested INTEGER NOT NULL DEFAULT 0;

-- Таблица движения сырья из теплицы
CREATE TABLE IF NOT EXISTS greenhouse_harvest_log (
    id           TEXT PRIMARY KEY,
    sort_id      TEXT NOT NULL REFERENCES flower_sorts(id) ON DELETE CASCADE,
    delta        INTEGER NOT NULL,  -- положительный = добавили, отрицательный = убрали
    reason       TEXT NOT NULL DEFAULT 'manual', -- 'manual'|'packaged'|'correction'
    note         TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Таблица упаковочных сессий (расширяем существующую packaging_log)
-- packaging_log уже есть. Добавляем поля:
ALTER TABLE packaging_log ADD COLUMN IF NOT EXISTS order_id TEXT REFERENCES orders(id);
ALTER TABLE packaging_log ADD COLUMN IF NOT EXISTS warehouse_confirmed INTEGER NOT NULL DEFAULT 0;
-- warehouse_confirmed=1 означает что пачки приняты на склад
```

### Миграция 011: расширение заказов

```sql
-- Файл: src-tauri/migrations/011_orders_extended.sql

-- Расширяем таблицу orders
ALTER TABLE orders ADD COLUMN IF NOT EXISTS customer_company   TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS delivery_address   TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS delivery_notes     TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS pack_count_ordered INTEGER NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS pack_count_ready   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS deadline_confirmed INTEGER NOT NULL DEFAULT 0;
-- deadline_confirmed=1 когда пользователь нажал "Подтвердить выполнение"

-- Расширяем order_items для цветочного режима
ALTER TABLE order_items ADD COLUMN IF NOT EXISTS sort_id       TEXT REFERENCES flower_sorts(id);
ALTER TABLE order_items ADD COLUMN IF NOT EXISTS pack_count    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE order_items ADD COLUMN IF NOT EXISTS stems_per_pack INTEGER NOT NULL DEFAULT 0;

-- Индекс для быстрого поиска по дедлайну
CREATE INDEX IF NOT EXISTS idx_orders_deadline ON orders(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_orders_status   ON orders(status);
```

### Миграция 012: app_settings (константы и настройки)

```sql
-- Файл: src-tauri/migrations/012_app_settings.sql

-- Универсальная таблица настроек (key-value с типом)
CREATE TABLE IF NOT EXISTS app_settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    value_type  TEXT NOT NULL DEFAULT 'string', -- 'string'|'number'|'bool'|'json'
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Начальные значения для цветочного режима
INSERT OR IGNORE INTO app_settings (key, value, value_type) VALUES
    ('flower.stems_per_pack',   '10',    'number'),
    ('flower.weight_per_pack',  '0.5',   'number'),
    ('flower.pricing_mode',     'pack',  'string'),
    ('flower.price_per_pack',   '0',     'number'),
    ('flower.price_per_stem',   '0',     'number'),
    ('ui.theme_seed',           '#6b7280','string'),
    ('ui.color_mode',           'dark',  'string'),
    ('ui.dock_order_flowers',   '["dashboard","greenhouse","warehouse","orders","analytics","settings","sync"]', 'json');
```

---

## 2. RUST — новые команды (src-tauri/src)

### 2.1 commands/greenhouse.rs (НОВЫЙ ФАЙЛ)

Команды для теплицы: добавление/удаление/редактирование сырья, загрузка фото, история движения.

```rust
// Добавить в src-tauri/src/commands/greenhouse.rs

// Сигнатуры команд (реализовать полностью):

/// Сохранить фото для сорта цветка.
/// Копирует файл в app_data_dir/flower_photos/{sort_id}.{ext}
/// Возвращает относительный путь.
#[tauri::command]
pub fn save_flower_photo(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    sort_id: String,
    source_path: String,   // абсолютный путь к исходному файлу
) -> Result<String, String>

/// Добавить запись в журнал движения сырья.
/// delta > 0 = поступление, delta < 0 = списание
#[tauri::command]
pub fn log_greenhouse_harvest(
    db: State<'_, Database>,
    sort_id: String,
    delta: i32,
    reason: String,   // "manual" | "packaged" | "correction"
    note: Option<String>,
) -> Result<(), String>

/// Получить журнал движения по конкретному сорту.
#[tauri::command]
pub fn get_harvest_log(
    db: State<'_, Database>,
    sort_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<HarvestLogEntry>, String>
```

**HarvestLogEntry** (добавить в events/types.rs):
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HarvestLogEntry {
    pub id: String,
    pub sort_id: String,
    pub sort_name: String,
    pub delta: i32,
    pub reason: String,
    pub note: Option<String>,
    pub created_at: String,
}
```

### 2.2 commands/orders.rs — дополнить существующий файл

Добавить следующие команды в существующий `commands/orders.rs`:

```rust
/// Обновить расширенные поля заказа (компания, адрес, кол-во пачек и т.д.)
#[tauri::command]
pub fn update_order_extended(
    db: State<'_, Database>,
    order_id: String,
    customer_company: Option<String>,
    delivery_address: Option<String>,
    delivery_notes: Option<String>,
    pack_count_ordered: Option<i32>,
) -> Result<(), String>

/// Подтвердить выполнение заказа (deadline_confirmed = 1).
/// Вызывается когда пользователь нажимает "Подтвердить" в уведомлении.
#[tauri::command]
pub fn confirm_order_deadline(
    db: State<'_, Database>,
    order_id: String,
) -> Result<(), String>

/// Получить заказы с истёкшим дедлайном, которые ещё не подтверждены.
/// Используется при старте приложения для показа уведомлений.
#[tauri::command]
pub fn get_overdue_unconfirmed_orders(
    db: State<'_, Database>,
) -> Result<Vec<Order>, String>

/// Проверить нехватку по складу для заказов.
/// Возвращает список {order_id, sort_id, sort_name, ordered, available, shortage}
#[tauri::command]
pub fn check_order_shortages(
    db: State<'_, Database>,
) -> Result<Vec<OrderShortage>, String>
```

**OrderShortage** (добавить в events/types.rs):
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrderShortage {
    pub order_id: String,
    pub customer_name: String,
    pub sort_id: String,
    pub sort_name: String,
    pub ordered_packs: i32,
    pub available_packs: i32,
    pub shortage: i32,      // ordered_packs - available_packs, если > 0 — дефицит
}
```

### 2.3 commands/app_settings.rs (НОВЫЙ ФАЙЛ)

```rust
/// Получить значение настройки по ключу.
#[tauri::command]
pub fn get_setting(db: State<'_, Database>, key: String) -> Result<Option<String>, String>

/// Установить значение настройки.
#[tauri::command]
pub fn set_setting(db: State<'_, Database>, key: String, value: String) -> Result<(), String>

/// Получить все настройки разом (для инициализации).
#[tauri::command]
pub fn get_all_settings(db: State<'_, Database>) -> Result<Vec<AppSetting>, String>
```

**AppSetting** struct:
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub value_type: String,
}
```

### 2.4 commands/version.rs — дополнить

В существующем файле `commands/version.rs` расширить ответ:

```rust
#[derive(Debug, serde::Serialize)]
pub struct VersionInfo {
    pub app_version: String,         // "0.0.7"
    pub db_schema_version: i32,      // последняя применённая миграция
    pub min_compatible_version: String, // "0.0.5" — с какой версии синхронизация корректна
}

#[tauri::command]
pub fn get_version_info(db: State<'_, Database>) -> Result<VersionInfo, String>
```

### 2.5 Регистрация всех новых команд в lib.rs

В `src-tauri/src/lib.rs` добавить в `invoke_handler`:
```rust
// Greenhouse
commands::greenhouse::save_flower_photo,
commands::greenhouse::log_greenhouse_harvest,
commands::greenhouse::get_harvest_log,
// Orders extended
commands::orders::update_order_extended,
commands::orders::confirm_order_deadline,
commands::orders::get_overdue_unconfirmed_orders,
commands::orders::check_order_shortages,
// App settings
commands::app_settings::get_setting,
commands::app_settings::set_setting,
commands::app_settings::get_all_settings,
// Version
commands::version::get_version_info,
```

### 2.6 Модуль уведомлений при старте (startup check)

В `lib.rs`, в блоке `.setup()`, ПОСЛЕ инициализации БД добавить:

```rust
// При старте — проверяем просроченные неподтверждённые заказы
// и эмитим событие "overdue-orders" на фронтенд
let h2 = app.handle().clone();
tauri::async_runtime::spawn(async move {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    // ... query db for overdue orders, emit event
    let _ = h2.emit("overdue-orders", overdue_list);
});
```

---

## 3. FRONTEND — TypeScript типы (src/lib/tauri/types.ts)

Добавить в конец файла `src/lib/tauri/types.ts`:

```typescript
// ============================================================
// Greenhouse harvest log
// ============================================================
export interface HarvestLogEntry {
  id: string;
  sort_id: string;
  sort_name: string;
  delta: number;
  reason: 'manual' | 'packaged' | 'correction';
  note?: string;
  created_at: string;
}

// ============================================================
// Orders extended
// ============================================================

// Расширить существующий Order:
// customer_company?: string
// delivery_address?: string
// delivery_notes?: string
// pack_count_ordered: number
// pack_count_ready: number
// deadline_confirmed: boolean

export interface OrderShortage {
  order_id: string;
  customer_name: string;
  sort_id: string;
  sort_name: string;
  ordered_packs: number;
  available_packs: number;
  shortage: number;
}

// ============================================================
// App settings
// ============================================================
export interface AppSetting {
  key: string;
  value: string;
  value_type: 'string' | 'number' | 'bool' | 'json';
}

// ============================================================
// Version info
// ============================================================
export interface VersionInfo {
  app_version: string;
  db_schema_version: number;
  min_compatible_version: string;
}
```

Также **расширить** существующий интерфейс `Order`:
```typescript
export interface Order {
  // ... существующие поля ...
  customer_company?: string;
  delivery_address?: string;
  delivery_notes?: string;
  pack_count_ordered: number;
  pack_count_ready: number;
  deadline_confirmed: boolean;
}
```

---

## 4. FRONTEND — Stores (src/lib/stores/)

### 4.1 Новый store: src/lib/stores/settings.ts

```typescript
// Writable store для app_settings.
// При инициализации загружает все настройки через get_all_settings.
// Предоставляет helpers: getSetting(key), setSetting(key, value).
// Также хранит константы цветочного режима как производные значения:
//   stemsPerPack, weightPerPack, pricingMode, pricePerPack, pricePerStem
```

### 4.2 Расширить src/lib/stores/orders.ts

Добавить:
- `shortages` — derived store: список OrderShortage (загружается через check_order_shortages)
- `overdueOrders` — список заказов с истёкшим дедлайном без подтверждения
- `confirmDeadline(orderId)` — вызов confirm_order_deadline
- `updateExtended(...)` — вызов update_order_extended

### 4.3 Расширить src/lib/stores/flowers.ts

Добавить:
- `harvestLog` — список HarvestLogEntry
- `logHarvest(sortId, delta, reason, note?)` — вызов log_greenhouse_harvest
- `loadHarvestLog(sortId?)` — загрузка истории
- `savePhoto(sortId, sourcePath)` — вызов save_flower_photo

---

## 5. НАВИГАЦИЯ — layout.svelte

**Файл:** `src/routes/+layout.svelte`

Порядок пунктов в доке для `preset === 'flowers'` (оставить как есть, только переименовать):
```
1. Dashboard      → /
2. Greenhouse     → /flowers      (Теплица — иконка IconGreenhouse)
3. Warehouse      → /inventory    (Склад — иконка IconWarehouse)
4. Orders         → /orders       (Заказы — иконка IconOrders)
5. Analytics      → /analytics    (Аналитика — иконка IconAnalytics)
6. Settings       → /settings
7. Sync           → modal
```

**Уведомления о просроченных заказах:** В `+layout.svelte` добавить слушатель события Tauri:
```typescript
import { listen } from '@tauri-apps/api/event';

onMount(async () => {
  const unlisten = await listen<Order[]>('overdue-orders', (event) => {
    if (event.payload.length > 0) {
      overdueOrders.set(event.payload);
      showOverdueModal = true;
    }
  });
  return unlisten;
});
```

Показывать модальное окно с текстом:
> «[N] заказ(ов) ожидали выполнения. Подтвердите статус каждого.»  
> Список заказов с кнопками «Подтвердить» / «Отложить».

---

## 6. ЭКРАН: ТЕПЛИЦА (/flowers)

**Файл:** `src/routes/flowers/+page.svelte`

### Что изменить (не переписывать с нуля, а дополнить):

**6.1 Карточки сортов — добавить фотографию**

Каждая тайл-карточка должна показывать:
- Фото цветка (если загружено) — верхняя половина карточки, object-fit: cover
- Если фото нет — цветной градиент по `color_hex`
- Название (`name`) крупно
- Сорт (`variety`) мелко
- Количество стеблей: `raw_stock` — крупное число + "шт."
- Количество упаковок: `pkg_stock` — меньше, с бейджем "уп."
- Кнопка **"Упаковать"** — главное действие карточки (открывает PackagingModal)

**6.2 Добавить кнопку загрузки фото в форму добавления/редактирования**

При создании/редактировании сорта — поле «Фото»:
```html
<input type="file" accept="image/*" onchange={handlePhotoSelect} />
<!-- или drag-and-drop зона -->
```
При выборе файла — вызвать `save_flower_photo(sort_id, source_path)` и обновить store.

**6.3 Форма добавления сорта — расширить поля:**
- Название вида (напр. «Лилия»)
- Сорт (напр. «White Heaven»)
- Цвет карточки (color picker)
- Начальное количество стеблей (число, по умолчанию 0)
- Цена закупки за стебель
- Цена продажи за упаковку
- Фото (необязательно)

**6.4 KPI-полоса вверху страницы:**
```
[ Всего стеблей: 3 450 ]  [ Упаковок: 124 ]  [ Сортов: 8 ]
```
Значения — реактивные derived из `$flowerSorts`.

**6.5 Фильтры и поиск:**
- Поиск по тексту (уже есть — оставить)
- Сортировка: «По названию А→Я», «По количеству ↓», «По количеству ↑»
- Вкладки: Все / Есть стебли / Есть упаковки

---

## 7. ЭКРАН: СКЛАД (/inventory)

**Файл:** `src/routes/inventory/+page.svelte`

Это главный экран для работы с упаковкой и готовым товаром. В режиме `flowers`:

### 7.1 Два режима вида (переключатель вверху):

**Режим А: «Упаковка» (Packaging)**
- Список сортов из теплицы (из `$flowerSorts`)
- Для каждого сорта показывает: фото, название, остаток стеблей, сколько можно упаковать
- Кнопка **«Упаковать»** → открывает PackagingModal
- KPI: Всего стеблей | Всего упаковок | Ожидает упаковки (заказы без готовых пачек)

**Режим Б: «Готовый товар» (Warehouse)**  
- Карточки упакованных пачек по сортам
- Для каждого сорта: фото, название, `pkg_stock` пачек, `pkg_stock * stems_per_pack` стеблей
- Кнопка **«Отправить заказчику»** → открывает выбор заказа

### 7.2 PackagingModal — полная переработка

**Файл:** `src/lib/components/flowers/PackagingModal.svelte`

Шаги в модале:
1. **Выбор сорта** (если открыт не из карточки) — список карточек-миниатюр
2. **Параметры упаковки:**
   - Показать: доступно стеблей = `raw_stock`
   - Стеблей в упаковке = берётся из настроек (`stemsPerPack`), можно изменить
   - Поле ввода: «Количество упаковок» (число)
   - Автоматически показать: «Спишется стеблей: N» и «Останется: M»
   - Максимум упаковок = `floor(raw_stock / stemsPerPack)` — показать рядом
3. **Привязка к заказу (необязательно):**
   - Кнопка «+ Привязать к заказу» → выпадающий список активных заказов
   - Если заказ выбран — показать имя клиента и сколько пачек ему нужно
4. **Кнопка «Упаковать и отправить на склад»**
   - Вызывает `package_flowers(sort_id, pack_count)`
   - Если выбран заказ — создаёт PackAssignment
   - Закрывает модал, обновляет store

### 7.3 Кнопки на карточках (hover/long-press)

На каждой карточке при наведении (desktop) или долгом нажатии (touch) появляются три кнопки:
- **✎ Редактировать** — открывает ItemDetailModal
- **⧉ Дублировать** — создаёт копию с новым ID
- **✕ Удалить** — подтверждение → удаление

Стиль кнопок: матовое стекло (glass morphism), мягкие тени, подстраиваются под оттенок карточки.

### 7.4 Аналитика склада (коллапсируемая панель вверху)

```
[ Скрыть ▲ ]
Всего пачек: 124 | Стеблей: 3 450 | Сортов с запасом: 6 | Дефицит по заказам: 2
```

### 7.5 Поиск и сортировка

- Поле поиска (уменьшить до 200px)
- Кнопки сортировки рядом:
  - **А→Я** / **Я→А** (по названию, поддержка кириллицы и латиницы)
  - **↑ Кол-во** / **↓ Кол-во** (по pkg_stock)
  - **↑ Цена** / **↓ Цена** (по sell_price_stem)

---

## 8. ЭКРАН: ЗАКАЗЫ (/orders)

**Файл:** `src/routes/orders/+page.svelte`

### 8.1 Структура экрана

**Верхняя панель (KPI):**
```
Всего заказов: 12 | Активных: 5 | Выполнено: 7 | Дефицит: ⚠ 2
```

**Вкладки статусов:** Все | Ожидают | В работе | Выполнены | Отменены

**Список заказов** — карточки:
- Имя клиента (крупно)
- Компания (если есть)
- Телефон + email
- Дата дедлайна + сколько времени осталось (в часах/днях)
- Количество заказанных пачек
- Количество готовых пачек / нужных → прогресс-бар
- **Флаг дефицита ⚠** — если `available_packs < ordered_packs` по любому сорту
- Статус (цветной бейдж)

### 8.2 Форма создания заказа

Кнопка **«+ Новый заказ»** → открывает боковую панель (slideout) с полями:

**Клиент:**
- Имя *
- Фамилия
- Компания
- Телефон *
- Email
- Адрес доставки
- Заметки о доставке

**Состав заказа (строки):**
- Каждая строка: выбрать сорт (dropdown с фото) → количество пачек → автоцена
- Кнопка «+ Добавить позицию»
- Показать итого: «N пачек / M стеблей»

**Дедлайн:**
- Поле дата + время (datetime-local input)
- Подсказка: «Когда нужно отдать заказ»

**Кнопка «Создать заказ»** → `create_order` + `add_order_items` + `update_order_extended`

### 8.3 Детальный просмотр заказа

При клике на карточку → открывается модальное окно (большое):

```
[Заголовок] Заказ #XXXX — Иван Петров
[Статус-бейдж]

Клиент:
  Имя: Иван Петров     Компания: ООО Ромашка
  Тел: +995 ...        Email: ivan@...
  Доставка: ул. Руставели 5

Состав заказа:
  +----------+---------+---------+---------+
  | Сорт     | Пачек   | Стеблей | Статус  |
  +----------+---------+---------+---------+
  | Лилия WH |   10    |   100   | ✓ Готово|
  | Роза Red |    5    |    50   | ⚠ -3 уп.|
  +----------+---------+---------+---------+
  Итого: 15 упаковок / 150 стеблей

Дедлайн: 12 апреля 2026, 10:00 (осталось 2 дня)
```

**Кнопки в модале:**
- «Изменить статус» (slider или кнопки)
- **«Распечатать» → PDF**
- «Удалить»

### 8.4 Печать заказа (PDF)

При нажатии «Распечатать»:
1. Открывается превью (window.print() или Tauri printToPDF)
2. Структура печатной формы:

```
╔══════════════════════════════════════╗
║        EXSUL — ЗАКАЗ #XXXX          ║
║        [дата создания]               ║
╠══════════════════════════════════════╣
║ КЛИЕНТ                               ║
║ Имя: _______  Компания: _______      ║
║ Тел: _______  Email: _______         ║
║ Адрес: _______                       ║
╠══════════════════════════════════════╣
║ СОСТАВ ЗАКАЗА                        ║
║ Сорт        Пачек  Стеблей  Цена     ║
║ Лилия WH     10     100     500 ₾    ║
║ Роза Red      5      50     250 ₾    ║
║ ─────────────────────────────────── ║
║ ИТОГО:       15     150     750 ₾    ║
╠══════════════════════════════════════╣
║ ДОСТАВКА                             ║
║ Адрес: _______  Заметки: _______     ║
╠══════════════════════════════════════╣
║ Дедлайн: 12 апреля 2026, 10:00       ║
║ Статус: В работе                     ║
╚══════════════════════════════════════╝
```

**Реализация печати:**
```typescript
// В orders/+page.svelte или компоненте OrderDetailModal
async function printOrder(order: Order) {
  // Генерировать HTML-строку с таблицей заказа
  // Открыть новое окно (window.open)
  // Вставить HTML + стили для печати
  // Вызвать window.print()
  // Закрыть окно
}
```

CSS для печати (в отдельном блоке `@media print`):
- Белый фон, чёрный текст
- Шрифт serif, 12pt
- Убрать всё кроме таблицы заказа
- Размер A4

### 8.5 Дефицит (shortage alert)

Если у заказа `shortage > 0` для любого сорта:
- На карточке заказа — красный бейдж ⚠ «Нехватка»
- В детальном виде — красная строка с указанием конкретного сорта и нехватки
- В KPI-полосе — счётчик «Дефицит: N заказов»

---

## 9. ЭКРАН: АНАЛИТИКА (/analytics)

**Файл:** `src/routes/analytics/+page.svelte`

### 9.1 Что убрать

В режиме `flowers`:
- Убрать все упоминания выручки, стоимости, маржи из основных виджетов
- Убрать столбцы «Цена», «Выручка», «Себестоимость» из основных таблиц

### 9.2 Что оставить / добавить

**KPI-полоса (только количество):**
```
[ Всего стеблей в теплице: 3 450 ]
[ Всего упаковок на складе: 124 ]
[ Упаковано сегодня: 30 ]
[ Отгружено сегодня: 15 ]
```

**Таблица по сортам:**
| Сорт | Стеблей в теплице | Упаковок на складе | (стеблей) | Упаковано всего |
|------|-------------------|-------------------|-----------|-----------------|
| Лилия WH | 450 | 18 | (180) | 340 |
| Роза Red | 200 | 5 | (50) | 120 |

_Стебли в скобках = pkg_stock × stemsPerPack_

**График: Динамика упаковки** (последние 7/14/30 дней)
- Данные из `packaging_log`
- Барный график: по оси X — дни, по оси Y — количество упакованных стеблей
- Реализовать через Canvas (уже есть спарклайн в дашборде — использовать тот же подход)

**Таблица: История упаковки** (последние N записей из packaging_log)
| Дата | Сорт | Упаковок | Стеблей |
|------|------|---------|---------|
| 10 апр | Лилия WH | 5 | 50 |

**Фильтр по датам:** «от → до» (уже есть в текущей аналитике — оставить)

---

## 10. ЭКРАН: ДАШБОРД (/)

**Файл:** `src/routes/+page.svelte`

### 10.1 Убрать с дашборда

В режиме `flowers`:
- Виджет с графиком выручки (chart) — заменить на график упаковки
- Убрать `totalRevenue` из отображения
- Секция «Цветы» как отдельный виджет — убрать (это теперь в Теплице)

### 10.2 Добавить

Виджет «Теплица — быстрый обзор»:
```
Стеблей: 3 450 | Упаковок: 124 | Сортов: 8
[Перейти в Теплицу →]
```

Виджет «Заказы»:
```
Активных: 5 | ⚠ Дефицит: 2
[Перейти к Заказам →]
```

Виджет «Склад»:
```
Готово к отгрузке: 124 уп.
[Перейти на Склад →]
```

---

## 11. НАСТРОЙКИ (/settings)

**Файл:** `src/routes/settings/+page.svelte`

### 11.1 Секция «Константы упаковки» (только в режиме flowers)

Уже частично есть. Расширить и сделать более понятной:

```
─── Параметры упаковки ───────────────────────

  Стеблей в одной упаковке:    [ 10 ] шт.
  Вес одной упаковки:          [ 0.5 ] кг
  
  Режим ценообразования:
  ○ За упаковку    ○ За стебель    ○ Смешанный
  
  Цена упаковки:     [ 500 ] ₾
  Цена за стебель:   [  50 ] ₾
  
  [ Сохранить константы ]
  
  Подсказка: При N стеблях можно сделать M упаковок
  (показывать расчёт в реальном времени на основе
   текущих данных из теплицы)
```

### 11.2 Резервная копия — открывать проводник

При нажатии «Экспорт резервной копии»:
```typescript
// После export_backup():
import { open as shellOpen } from '@tauri-apps/plugin-shell';
// Открыть папку с файлом резервной копии
await shellOpen(backupDir);
// Или использовать tauri-plugin-opener
```

### 11.3 Стартовая тема — монохромная

При первом запуске seedColor должен быть `#6b7280` (нейтральный серый).  
Проверить в `src/lib/stores/theme.ts` — там где читается из localStorage:
```typescript
// Если значение не сохранено — использовать серый, не зелёный
const DEFAULT_SEED = '#6b7280';
```

### 11.4 Dropdown (GlassDropdown) — тема

**Файл:** `src/lib/components/common/GlassDropdown.svelte`

Компонент сейчас остаётся белым. Исправить: применять CSS переменные темы:
```css
.dropdown-menu {
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  color: var(--color-on-surface);
  /* убрать все хардкод цвета: white, #fff, rgb(255...) */
}
```

---

## 12. СИНХРОНИЗАЦИЯ И ВЕРСИОНИРОВАНИЕ

### 12.1 Отображение версии

В SyncModal и/или в настройках показывать:
```
Эта программа: v0.0.7 (схема БД: 12)
Подключённые устройства:
  • MacBook Анны  v0.0.6  ✓ совместим
  • iPad Давида   v0.0.5  ⚠ частичная совместимость
```

**Логика совместимости:**
- Если версия ≥ `min_compatible_version` → "совместим"
- Если версия < `min_compatible_version` → "несовместим, синхронизация невозможна"
- Промежуток → "частичная совместимость (новые функции недоступны)"

### 12.2 В commands/version.rs

```rust
pub fn get_version_info(db: ...) -> VersionInfo {
    VersionInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        db_schema_version: get_current_schema_version(db),
        min_compatible_version: "0.0.5".to_string(),
    }
}
```

Версия берётся из `Cargo.toml` через `env!("CARGO_PKG_VERSION")`.

---

## 13. ВАЛЮТА

В настройках и по всему приложению:
- Убрать символ рубля (₽)
- Использовать Georgian Lari (₾) по умолчанию, или то что выбрано в настройках
- В `src/lib/stores/currency.ts` проверить дефолт — должно быть GEL (₾), не RUB

В `CURRENCIES` убедиться что есть:
```typescript
export const CURRENCIES = [
  { code: 'GEL', symbol: '₾', label: 'Georgian Lari' },
  { code: 'USD', symbol: '$', label: 'US Dollar' },
  { code: 'EUR', symbol: '€', label: 'Euro' },
  // ...
];
```

---

## 14. i18n — ключи перевода

### Новые ключи для src/lib/i18n/en.json и ru.json

```json
{
  "nav_greenhouse": "Greenhouse",
  "nav_warehouse": "Warehouse",
  "page_greenhouse_title": "Greenhouse",
  "page_greenhouse_subtitle": "Raw flower stock — stems ready for packing",
  "page_warehouse_title": "Warehouse",
  "page_warehouse_subtitle": "Packed flowers ready for delivery",
  "greenhouse_add_sort": "Add flower sort",
  "greenhouse_photo": "Photo",
  "greenhouse_initial_stock": "Initial stems",
  "warehouse_pack_action": "Pack flowers",
  "warehouse_packaging_tab": "Packaging",
  "warehouse_ready_tab": "Ready stock",
  "order_customer_company": "Company",
  "order_delivery_address": "Delivery address",
  "order_delivery_notes": "Delivery notes",
  "order_pack_count": "Packs ordered",
  "order_shortage_alert": "Stock shortage for this order",
  "order_print": "Print order",
  "order_confirm_deadline": "Confirm completion",
  "analytics_stems_total": "Total stems",
  "analytics_packs_total": "Total packs",
  "analytics_packed_today": "Packed today",
  "analytics_shipped_today": "Shipped today",
  "analytics_packing_history": "Packing history",
  "settings_stems_per_pack": "Stems per pack",
  "settings_weight_per_pack": "Pack weight (kg)",
  "settings_pack_constants": "Pack constants",
  "version_compatible": "Compatible",
  "version_partial": "Partial compatibility",
  "version_incompatible": "Incompatible"
}
```

Русский вариант (ru.json):
```json
{
  "nav_greenhouse": "Теплица",
  "nav_warehouse": "Склад",
  "page_greenhouse_title": "Теплица",
  "page_greenhouse_subtitle": "Сырьё — стебли, готовые к упаковке",
  "page_warehouse_title": "Склад",
  "page_warehouse_subtitle": "Упакованные цветы, готовые к отгрузке",
  "greenhouse_add_sort": "Добавить сорт",
  "greenhouse_photo": "Фото",
  "greenhouse_initial_stock": "Начальный запас (шт.)",
  "warehouse_pack_action": "Упаковать",
  "warehouse_packaging_tab": "Упаковка",
  "warehouse_ready_tab": "Готово к отгрузке",
  "order_customer_company": "Компания",
  "order_delivery_address": "Адрес доставки",
  "order_delivery_notes": "Заметки о доставке",
  "order_pack_count": "Заказано упаковок",
  "order_shortage_alert": "Нехватка товара по заказу",
  "order_print": "Распечатать заказ",
  "order_confirm_deadline": "Подтвердить выполнение",
  "analytics_stems_total": "Всего стеблей",
  "analytics_packs_total": "Всего упаковок",
  "analytics_packed_today": "Упаковано сегодня",
  "analytics_shipped_today": "Отгружено сегодня",
  "analytics_packing_history": "История упаковки",
  "settings_stems_per_pack": "Стеблей в упаковке",
  "settings_weight_per_pack": "Вес упаковки (кг)",
  "settings_pack_constants": "Параметры упаковки",
  "version_compatible": "Совместима",
  "version_partial": "Частичная совместимость",
  "version_incompatible": "Несовместима"
}
```

---

## 15. ПОРЯДОК РЕАЛИЗАЦИИ (приоритет)

```
Фаза 1 — База (критично для работы):
  [ ] Миграции 010, 011, 012
  [ ] commands/greenhouse.rs + регистрация
  [ ] Расширить commands/orders.rs (update_order_extended, confirm_deadline, check_shortages)
  [ ] commands/app_settings.rs + регистрация
  [ ] Обновить types.ts (новые интерфейсы)
  [ ] Исправить дефолт валюты на GEL (₾)
  [ ] Исправить дефолт темы на серый (#6b7280)
  [ ] Исправить GlassDropdown (тема)

Фаза 2 — Теплица:
  [ ] Добавить поле photo_path в FlowerSort
  [ ] Обновить /flowers: фото в карточках, форма с начальным остатком
  [ ] Добавить загрузку фото (save_flower_photo)

Фаза 3 — Склад:
  [ ] Переработать /inventory в режиме flowers: два режима (Упаковка / Готово)
  [ ] PackagingModal: привязка к заказу, максимальный расчёт
  [ ] Hover-кнопки на карточках (редакт./дубль/удал.)
  [ ] Сортировка и поиск (кириллица + латиница)

Фаза 4 — Заказы:
  [ ] Расширить форму создания заказа (компания, адрес, пачки)
  [ ] Детальный просмотр заказа с нехваткой
  [ ] Печать заказа (window.print + CSS @media print)
  [ ] Уведомления о просроченных заказах (Tauri event)

Фаза 5 — Аналитика:
  [ ] Убрать выручку из основных виджетов (flowers mode)
  [ ] Добавить количественные KPI
  [ ] График динамики упаковки

Фаза 6 — Настройки + Версионирование:
  [ ] Секция «Параметры упаковки» (расширить существующую)
  [ ] Открытие проводника после экспорта резервной копии
  [ ] get_version_info команда
  [ ] Отображение версий в SyncModal
  [ ] i18n: добавить все новые ключи

Фаза 7 — Дашборд + полировка:
  [ ] Обновить дашборд: виджеты теплицы/склада/заказов
  [ ] Убрать лишнее из дашборда (режим flowers)
```

---

## 16. ОГРАНИЧЕНИЯ И ВАЖНЫЕ ЗАМЕЧАНИЯ

1. **Не ломать существующую синхронизацию (CRDT).** Все новые таблицы не должны мешать event-sourcing механизму. Новые команды НЕ должны писать в таблицу `events` напрямую, если не используют HLC.

2. **Миграции — только additive.** Никогда не удалять столбцы или таблицы в миграциях. Только `ADD COLUMN IF NOT EXISTS`, `CREATE TABLE IF NOT EXISTS`.

3. **Режим flowers — изолированный.** Все изменения в /flowers и специфика упаковки должны быть защищены проверкой `$preset === 'flowers'`. Другие режимы не должны пострадать.

4. **Хранение фото.** Фотографии хранятся локально в `app_data_dir/flower_photos/`. В БД хранится только относительный путь. При синхронизации фото НЕ передаются.

5. **Печать — только через браузер.** Не использовать сторонние PDF библиотеки. `window.print()` с CSS `@media print` достаточно.

6. **Уведомления о дедлайнах.** Проверять при старте приложения (не в фоне). Tauri не поддерживает background tasks без явного запуска.

7. **Дефицит по заказам** — рассчитывается на лету (`check_order_shortages`), не хранится в БД.

---

## 17. ТЕСТОВЫЕ СЦЕНАРИИ

После реализации проверить вручную:

1. **Сценарий «Новый сорт»:** Создать сорт «Лилия Белая», загрузить фото, указать 1000 стеблей. Убедиться что отображается в Теплице и на Дашборде.

2. **Сценарий «Упаковка»:** Нажать «Упаковать» на Лилии Белой, ввести 5 пачек (10 шт/пачка = 50 стеблей). Проверить: raw_stock стал 950, pkg_stock стал 5.

3. **Сценарий «Заказ с дефицитом»:** Создать заказ на 200 пачек Лилии Белой. На складе 5 пачек → должен показаться ⚠ дефицит 195 пачек.

4. **Сценарий «Печать заказа»:** Открыть заказ → «Распечатать» → появляется аккуратная таблица.

5. **Сценарий «Дедлайн»:** Создать заказ с дедлайном в прошлом. Перезапустить приложение → появляется уведомление.

6. **Сценарий «Тема»:** При первом запуске тема серая, не зелёная. Dropdown в настройках использует тему, не белый фон.
