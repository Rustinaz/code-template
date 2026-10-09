# Rustinaz

**قالب اپلیکیشن کراسپلتفرم با رویکرد «اول راست».** یک کدبیس که یک اپلیکیشن واقعی برای
**لینوکس، ویندوز، مک، اندروید، iOS و وب** می‌سازد — با یک UI مشترک که یک‌بار در egui نوشته
می‌شود، یک لایهٔ نازک مخصوص هر پلتفرم برای چیزهایی که واقعاً تفاوت دارند، و یک بک‌اند
هم‌بسته در همان ورک‌اسپیس.

> 🇬🇧 **English version:** [README.md](README.md)

---

## Rustinaz چیست

هیچ **سورس کد** جاوا یا کاتلین، هیچ layoutی از جنس XML، هیچ Swift و هیچ رابط کاربری
HTML/CSS در این پروژه وجود ندارد. اپ اندروید یک فایل `.so` راست است؛ اپ وب همان راست است
که به WASM کامپایل شده. اسکریپت‌های بیلد Gradle پروژهٔ اندروید با Kotlin DSL نوشته شده‌اند و
چند فایل منبع XML (manifest، تم، آیکون لانچر) وجود دارد، اما هیچ‌کدام کد اپلیکیشن نیستند و
هیچ‌کدام یک پیکسل هم رسم نمی‌کنند — APK صفر فایل `.class` دارد.

Rustinaz یک بک‌اند کوچک و کاملاً راست هم در همان ورک‌اسپیس دارد (`apps/server`) که تایپ‌های
درخواست و پاسخش یک‌بار در `shared` تعریف می‌شوند و هر دو سر (سرور و کلاینت) از همان import
می‌کنند؛ پس سرور و کلاینت در همین یک پوشه نوشته و اجرا می‌شوند.

بیشتر قالب‌های «کراسپلتفرم» یکی از این سه کار را می‌کنند: یا فقط دسکتاپ را پوشش می‌دهند، یا
یک WebView را پشت یک پوستهٔ نیتیو قایم می‌کنند، یا مجبورت می‌کنند UI را برای هر پلتفرم جدا
بنویسی. Rustinaz شرط را برعکس می‌بندد — **UI را با راست بنویس، همان درخت ویجت‌ها را به هر
هدف بفرست، و فقط بخش‌هایی که واقعاً وابسته به سیستم‌عامل‌اند (پنجره، تم، اطلاعات دستگاه،
دسترس‌پذیری، JNI) را در crateهای مخصوص هر پلتفرم نگه دار.** یک زبان، یک سیستم بیلد، یک UI.

## ویژگی‌ها

- **شش پلتفرم، یک UI.** egui در همه‌جا — دسکتاپ، اندروید، iOS و مرورگر.
- **اندروید بدون جاوا یا کاتلین.** منیفست `android.app.NativeActivity` و کتابخانهٔ `cdylib`
  راست را لود می‌کند؛ APK صفر فایل `.class` دارد.
- **تفکیک «منطق مشترک / مخصوص پلتفرم» که می‌شود دیدش.** `crates/ui` کتابخانهٔ کامپوننت مشترک
  است و `crates/platform/*` فقط چیزهایی را دارد که باید به سیستم‌عامل گفته شوند.
- **بک‌اند در همان مخزن.** `shared::api` تنها قرارداد سیم است، پس کلاینت و سرور از هم دور
  نمی‌افتند.
- **قبل از هر تنظیمی اجرا می‌شود.** روی دسکتاپ `cargo run`؛ و `scripts/dev.sh` هم بک‌اند و هم
  کلاینت را با هم بالا می‌آورد.
- **مهندسی صادقانه.** هر دروازهٔ کیفیت یک دستور است که خودت می‌توانی اجرا کنی، و همین
  README دقیقاً می‌گوید چه چیزی تست شده و چه چیزی نه.

## فهرست مطالب

- [Rustinaz چیست](#rustinaz-چیست)
- [ویژگی‌ها](#ویژگیها)
- [دریافت کد](#دریافت-کد)
- [پیش‌نیازها](#پیشنیازها)
- [گام ۱: اجرا روی لینوکس](#گام-۱-اجرا-روی-لینوکس)
- [گام ۲: حلقهٔ دسکتاپ + اندروید](#گام-۲-حلقهٔ-دسکتاپ--اندروید)
- [سه دستور مهم](#سه-دستور-مهم)
- [نحوهٔ نوشتن UI](#نحوهٔ-نوشتن-ui)
- [بک‌اند، در همان ورک‌اسپیس](#بکاند-در-همان-ورکاسپیس)
- [ساختار مخزن](#ساختار-مخزن)
- [کار در IDE](#کار-در-ide)
- [بیلد هر پلتفرم](#بیلد-هر-پلتفرم)
- [پکیج اندروید](#پکیج-اندروید)
- [تست و دروازه‌های کیفیت](#تست-و-دروازههای-کیفیت)
- [چه چیزی تست شده و چه چیزی نه](#چه-چیزی-تست-شده-و-چه-چیزی-نه)
- [تغییر نام قالب به اپ خودت](#تغییر-نام-قالب-به-اپ-خودت)
- [مشارکت و گزارش آنچه یافتی](#مشارکت-و-گزارش-آنچه-یافتی)
- [نویسنده و راه ارتباطی](#نویسنده-و-راه-ارتباطی)
- [مجوز](#مجوز)

---

## دریافت کد

```bash
git clone git@github.com:Rustinaz/code-template.git
cd code-template
```

فایل `Cargo.lock` عمداً کامیت شده تا اولین بیلدت دقیقاً همان نسخه‌های وابستگی‌ای را بگیرد که
قالب با آن‌ها تست شده است.

اگر به‌جای clone یک فایل tar می‌خواهی، پوشه را بدون خروجی بیلد آرشیو کن. `tar` مقدار
`--exclude` را با مسیرِ *ذخیره‌شده* مقایسه می‌کند، پس باید مسیرها را کامل و نسبت به جایی که
دستور اجرا می‌شود بنویسی؛ یک `--exclude='./target'` وقتی آرشیو از پوشهٔ والد ساخته می‌شود
بی‌صدا هیچ‌چیز را نمی‌گیرد:

```bash
tar --exclude='code-template/target' \
    --exclude='code-template/android-app/build' \
    --exclude='code-template/android-app/.gradle' \
    --exclude='code-template/apps/example/dist' \
    -czf code-template.tar.gz code-template
```

خروجی باید حدوداً ۲۰۰ و چند کیلوبایت باشد. اگر چند ده مگابایت شد، یعنی excludeها نگرفته‌اند.

هرگز `target/` (چند گیگابایت کش بیلد)، `apps/example/dist/` (خروجی Trunk) و
`android-app/build/` را نفرست. هر سه تولیدشده‌اند و هر سه در `.gitignore` هستند.

## پیش‌نیازها

| برای بیلدِ | چه چیزی لازم است |
| --- | --- |
| دسکتاپ لینوکس، ویندوز، مک | یک toolchain راست (`rustup`). چیز دیگری نه. |
| اندروید | مورد بالا، به‌علاوهٔ Android SDK همراه با NDK (نسخهٔ r23 یا بالاتر). Android Studio راه ساده است. |
| iOS | مک با Xcode، با SDK نسخهٔ iOS 12 یا بالاتر. |
| وب | مورد بالا، به‌علاوهٔ `cargo install --locked trunk`. |

هدف‌های راست برای اندروید و وب:

```bash
rustup target add \
  aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android \
  wasm32-unknown-unknown
```

## گام ۱: اجرا روی لینوکس

```bash
cd code-template
cargo run --package example-app
```

یک پنجره با رابط egui و تم Material 3 باز می‌شود. برای لاگ دیباگ `-d` را اضافه کن:

```bash
cargo run --package example-app -- --debug
cargo run --package example-app -- --help
```

خط فرمان این گزینه‌ها را می‌پذیرد: `--platform`، `--framework`، `--debug/-d`،
`--config/-c PATH` و `--server URL`. سوئیچ `--server` کلاینت را به بک‌اند هم‌بسته وصل می‌کند و
[پایین‌تر](#بکاند-در-همان-ورکاسپیس) توضیح داده شده است.

برای اجرای کلاینت و بک‌اند با هم، در همان بخش سراغ `scripts/dev.sh` برو.

## گام ۲: حلقهٔ دسکتاپ + اندروید

این دقیقاً همان گردش‌کاری است که ساختار پروژه حول آن چیده شده: یک‌بار ویرایش کن، در هر دو
ببین.

**ترمینال ۱ — دسکتاپ، با هر ذخیره دوباره بیلد و بازاجرا می‌شود**

```bash
cargo install cargo-watch          # یک‌بار
cargo watch -x "run --package example-app"
```

**ترمینال ۲ — اندروید، با هر ذخیره دوباره بیلد و نصب می‌شود**

```bash
source scripts/android-env.sh      # clang مربوط به NDK را جایی می‌گذارد که Cargo پیدایش کند
cargo watch -x "build --package example-app --target aarch64-linux-android"
```

این کار `libexample_app.so` را به‌روز نگه می‌دارد. برای رساندنش به دستگاه، یا دوباره Gradle را
اجرا کن (`./gradlew installDebug`) یا کتابخانهٔ نو را push کن و اکتیویتی را ری‌استارت کن:

```bash
adb install -r android-app/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.rustcrossplatform/android.app.NativeActivity
```

بیلد مجدد کدی که از قبل نصب شده بخش کند ماجراست، پس یک حلقهٔ سریع معمولاً این‌طور است: ویرایش
کن، `cargo watch` فایل `.so` را دوباره می‌سازد، و وقتی واقعاً خواستی روی دستگاه ببینی
`./gradlew installDebug` را دوباره بزن. برای یک ABI این کار چند ثانیه است:

```bash
cd android-app && ./gradlew installDebug -PandroidAbi=arm64-v8a
```

**ترمینال ۳ — وب، اختیاری**

```bash
scripts/build-web.sh --serve       # روی http://localhost:8080 و با هر ذخیره دوباره بیلد می‌کند
```

`scripts/android-env.sh` فقط متغیرها را export می‌کند و چیزی بیلد نمی‌کند. sourc کردنش در یک
ترمینال روی بقیه اثر ندارد؛ برای همین ترمینال ۲ هم باید خودش آن را sourc کند.

**بک‌اند، اگر از آن استفاده می‌کنی.** `scripts/dev.sh` آن را کنار کلاینت دسکتاپ بالا می‌آورد.
برای اجرای تنهای‌اش — مثلاً برای امولاتور — از `cargo server` استفاده کن (کوتاه‌شدهٔ
`cargo run --package server`)؛ [بخش بک‌اند](#بکاند-در-همان-ورک‌اسپیس) را ببین.

## سه دستور مهم

```bash
# ۱. همه‌چیز، هر crate و هر هدف را چک کن
cargo check --workspace --all-targets

# ۲. تست‌ها را اجرا کن
cargo test --workspace --all-targets

# ۳. لینت، با هشدارها به‌جای خطا (warnings as errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

و فرمت:

```bash
cargo fmt --all
```

نام‌های کوتاه در `.cargo/config.toml` تعریف شده‌اند:

| نام کوتاه | معادل |
| --- | --- |
| `cargo c` | `check --workspace --all-targets` |
| `cargo t` | `test --workspace --all-targets` |
| `cargo b` / `cargo br` | `build --workspace` / `--release` |
| `cargo f` | `fmt --all` |
| `cargo l` | `clippy --workspace --all-targets` |
| `cargo check-all` | `check --workspace --all-targets --all-features` |
| `cargo server` | `run --package server` |
| `cargo deps` | `tree --workspace --depth 2` |
| `cargo docs` | `doc --workspace --no-deps` |

هیچ‌کدام هم‌نام یک زیردستور واقعی Cargo نیستند و این عمدی است: Cargo نام کوتاهی را که با یک
زیردستور داخلی هم‌نام باشد به *خودش* تبدیل می‌کند، پس `fmt = "fmt --all"` باعث می‌شود
`cargo fmt` بی‌نهایت داخل خودش بازگشت کند. برای همین `f` به‌جای `fmt` و `l` به‌جای `clippy`.

## نحوهٔ نوشتن UI

هر پلتفرم همان ویجت‌ها را با همان کد egui رسم می‌کند. هیچ UI جداگانه‌ای برای هر پلتفرم نیست که
لازم باشد هم‌گام نگه داشته شود، چون فقط یک UI وجود دارد.

**تفکیکی که دنبالش بودی — منطق مشترک به‌علاوهٔ بخش‌های مخصوص پلتفرم — در crateهاست:**

```
crates/ui/          UI مشترک: توکن‌های تم، ریاضیات چیدمان، کامپوننت‌ها، ترِیت PlatformUi
crates/platform/*   یک crate برای هر پلتفرم: چیزهایی که واقعاً به سیستم‌عامل نیاز دارند
```

`crates/ui/src/components/` کتابخانهٔ کامپوننت مشترک است. هر کامپوننت همان شکل دو-بخشی را
دارد:

۱. **منطق خالص** — builder، استراکت حالت، محاسبات اندازه و شعاع گوشه. هیچ تایپ egui‌ای ندارد،
   پس با خاموش‌بودن feature `egui` هم کامپایل و تست می‌شود.
۲. **رندر** — متد `show(&mut egui::Ui)` که پشت `#[cfg(feature = "egui")]` قرار دارد.

همین چیزی است که باعث می‌شود `cargo check -p ui --no-default-features` موفق شود و همین است
که به crateهای پلتفرم اجازه می‌دهد فقط برای توکن‌ها و تعریف ترِیت‌ها به `ui` وابسته باشند،
بدون کشیدن یک رندرر.

**crateهای پلتفرم فقط چیزی را دارند که واقعاً باید به سیستم‌عامل گفته شود.** عنوان پنجره،
پیش‌فرض‌های تم، اطلاعات دستگاه، اعلام‌های دسترس‌پذیری:

```rust
// crates/platform/android/src/lib.rs
impl PlatformUi for AndroidPlatformUi {
    fn platform(&self) -> Platform { Platform::Android }
    fn theme(&self) -> PlatformTheme { material3_theme() }
    fn announce_for_accessibility(&self, text: &str) {
        // فراخوانی واقعی JNI به android.view.View.announceForAccessibility
    }
}
```

`UiBuilder` همه را به هم وصل می‌کند و `.platform(Platform::Android)` چیزی است که پلتفرم را روی
دستگاه اصلاح می‌کند، چون مقدار پخته‌شده در فایل کانفیگ، مقدار همان ماشینی است که بیلد کرده:

```rust
let context = UiBuilder::new()
    .platform(Platform::Android)      // دستگاه، نه ماشینی که کامپایل کرده
    .device_info(DeviceInfo::current())
    .build()?;
```

### نقطهٔ ورود اندروید

`android-activity` نسخهٔ 0.6 تابعی با این امضای دقیق می‌خواهد، پس اپ نمونه آن را کنار بقیه
چیزها، در `apps/example/src/lib.rs` تعریف می‌کند:

```rust
#[cfg(target_os = "android")]
#[no_mangle]
#[allow(improper_ctypes_definitions)] // AndroidApp یک هندل راست است و از C نمی‌آید
pub extern "C" fn android_main(app: platform_android::AndroidApp) {
    platform_android::install_app(app.clone());
    // ...
}
```

### نقطهٔ ورود وب

مرورگر `main` ندارد، پس `WebRunner` مربوط به eframe از یک هوک
`#[wasm_bindgen(start)]` شروع می‌شود. کانواسش را با id پیدا می‌کند، برای همین `index.html`
باید عنصری هم‌نام داشته باشد:

```rust
#[cfg(target_arch = "wasm32")]
pub const WEB_CANVAS_ID: &str = "the_canvas_id";
```

`run()` دسکتاپ و `run_web()` وب عمداً ناسازگارند: `run_native` به پنجرهٔ سیستم‌عامل نیاز دارد و
`WebRunner` به document، پس هیچ‌کدام برای هدف آن یکی کامپایل نمی‌شود و هرکدام با `#[cfg]`
کنترل می‌شوند.

`src/main.rs` یک باینری نازک است که روی دسکتاپ `example_app::run()` را صدا می‌زند و زیر wasm
به `unreachable!()` می‌رسد. نگه‌داشتن اپ در کتابخانه همین است که اجازه می‌دهد همان کد هم
باینری دسکتاپ باشد، هم `.so` اندروید، هم `.wasm` مرورگر و هم `.a` آی‌اواس.

## بک‌اند، در همان ورک‌اسپیس

قالب یک بک‌اند به‌صورت یک member معمولی ورک‌اسپیس (`apps/server`) عرضه می‌کند، نه یک پروژهٔ
جدای سبک `npm`؛ پس سرور و کلاینت در یک‌جا نوشته و اجرا می‌شوند. بک‌اند عمداً کم‌وابستگی است:
راست خالص، یک ترد به‌ازای هر اتصال، I/O بلاکینگ و بدون async runtime. این کار خواندنش را ساده
می‌کند و مهم‌تر، کل ورک‌اسپیس را بدون نیاز به toolchain زبان C کراس‌کامپایل نگه می‌دارد.

**قرارداد یک‌بار نوشته می‌شود.** مسیر endpointها و تایپ‌های `serde` در
`crates/shared/src/api/` هستند:

```rust
// crates/shared/src/api/mod.rs
pub mod endpoints {
    pub const HEALTH: &str = "/api/health";
    pub const USERS: &str = "/api/users";
    pub const USERS_SEARCH: &str = "/api/users/search";
}
```

هم `apps/server` و هم `apps/example` این ماژول را import می‌کنند، پس دو طرف از هم دور نمی‌افتند:
نام یک فیلد را عوض کن و سر دیگر دیگر کامپایل نمی‌شود.

**سرور** هم دست‌نویس است. `apps/server/src/http.rs` یک درخواست HTTP/1.1 را پارس و پاسخ را
می‌نویسد، `apps/server/src/lib.rs` ثابت‌های `endpoints` را به handlerها نگاشت می‌کند و
`main.rs` یک باینری `clap` است:

```bash
cargo run --package server                          # 127.0.0.1:8080، با چند کاربر نمونه
cargo run --package server -- --bind 0.0.0.0:8080   # قابل‌دسترس از امولاتور یا شبکهٔ محلی
cargo run --package server -- --no-demo             # شروع با انبار کاربر خالی
```

**سمت کلاینت** `shared::services::HttpNetworkService` است، یک کلاینت HTTP/1.1 بلاکینگ و کوچک
روی `std::net`. با `#[cfg(not(target_arch = "wasm32"))]` کنترل می‌شود و فقط `http://` حرف
می‌زند: کشیدن یک پشتهٔ TLS به یک بیلد موبایل کراس‌کامپایل برای یک بک‌اند محلی نمی‌ارزد. چون
از `reqwest` استفاده نمی‌کند، به feature `network` هم نیاز ندارد و بیلدهای اندروید و وب از
TLS پاک می‌مانند.

**هر دو را با یک دستور اجرا کن:**

```bash
scripts/dev.sh                                # بک‌اند روی 127.0.0.1:8080، سپس کلاینت دسکتاپ
scripts/dev.sh --release
scripts/dev.sh --bind 0.0.0.0:8080            # کلاینت‌های امولاتور یا شبکهٔ محلی را هم بپذیر
scripts/dev.sh --server-only                  # فقط بک‌اند، در پیش‌زمینه
scripts/dev.sh --client-only --server http://127.0.0.1:8080
```

`dev.sh` بک‌اند را بالا می‌آورد، صبر می‌کند تا پورتش واقعاً جواب بدهد، کلاینت را با `--server`
به آن وصل می‌کند، و وقتی کلاینت خارج شد بک‌اند را می‌کشد — از جمله با Ctrl-C. این همان حلقهٔ
«با هم نوشتن، با هم اجرا کردن» است: یک ترمینال، یک دستور.

سوئیچ `--server URL` در کلاینت، `network.base_url` را برای همان اجرا بازنویسی می‌کند. مقدار
پیش‌فرض داخلی از قبل آدرس همین بک‌اند هم‌بسته است، پس `cargo run --package example-app` ساده
آن را پیدا می‌کند:

```bash
cargo run --package example-app -- --server http://192.168.1.20:8080
```

روی اندروید، `127.0.0.1` خودِ دستگاه است، پس قبل از اجرای اپ پورت میزبان را فوروارد کن:

```bash
adb reverse tcp:8080 tcp:8080    # localhost:8080 دستگاه -> همین ماشین
```

اپ نمونه یک پنل کوچک **Backend** می‌کشد که می‌تواند هر endpoint را بدون بلاک‌کردن UI صدا
بزند: درخواست‌ها روی یک worker thread اجرا می‌شوند و نتیجه را از یک کانال `mpsc` برمی‌گردانند،
پس egui هیچ‌وقت روی شبکه معطل نمی‌ماند. مرورگر سوکت خام ندارد، بنابراین بیلد وب به‌جای تظاهر
به اتصال، یک placeholder توضیحی نشان می‌دهد؛ یک اپ وب واقعی همان endpointها را با `fetch`
صدا می‌زند.

کد HTTP/1.1 عمداً دست‌نویس است. وقتی از آن بزرگ‌تر شدی، درزِ تعویض یک فایل است: روتینگ در
`apps/server/src/lib.rs`، جایی که فریم‌ورکی مثل `axum` بدون تغییر DTOها یا کلاینت جا می‌افتد.

## ساختار مخزن

```
code-template/
├── Cargo.toml                  منیفست ورک‌اسپیس: اعضا، نسخه‌های مشترک وابستگی‌ها
├── Cargo.lock                  عمداً کامیت شده
├── rust-toolchain.toml         stable + rustfmt/clippy را پین می‌کند تا ادیتور و CI یکی باشند
├── .editorconfig               قواعد مستقل از ادیتور: UTF-8 / LF / تودرتویی
├── .cargo/config.toml          تنظیمات شبکه و نام‌های کوتاه؛ بدون linker ثابت‌شده
│
├── apps/
│   ├── example/                اپی که واقعاً ویرایشش می‌کنی
│   │   ├── src/lib.rs          run()، run_web()، android_main() و UI
│   │   ├── src/backend.rs      پنل بک‌اند کلاینت (نیتیو) و placeholder وب
│   │   ├── src/main.rs         باینری نازک دسکتاپ
│   │   ├── index.html          پوستهٔ وب؛ همان <canvas> که eframe سوارش می‌شود
│   │   ├── Trunk.toml          کانفیگ بیلد وب
│   │   └── dist/               خروجی Trunk (تولیدشده، در gitignore)
│   └── server/                 بک‌اند هم‌بسته، راست خالص
│       ├── src/lib.rs          روتینگ: ثابت‌های endpoint -> handlerها
│       ├── src/http.rs         پارس درخواست و نوشتن پاسخ HTTP/1.1
│       └── src/main.rs         باینری clap (`--bind`، `--no-demo`)
│
├── crates/
│   ├── shared/                 منطق کسب‌وکار: دامنه، کانفیگ، خطاها، سرویس‌ها
│   │   ├── src/api/            قرارداد سیم مشترک با apps/server
│   │   └── src/services/       ریپازیتوری‌ها، auth، تنظیمات، شبکه (+ کلاینت http، stub)
│   ├── ui/                     تم، چیدمان، ناوبری، کامپوننت‌ها، PlatformUi
│   │   └── src/components/     کتابخانهٔ کامپوننت مشترک
│   ├── resources/              رشته‌ها، رنگ‌ها، تم‌ها، assetها، بارگذاری‌شده در زمان اجرا
│   ├── build-config/           نسخه، هدف و کانفیگ امضا
│   └── platform/
│       ├── linux/              پنجره، تم، اطلاعات دستگاه
│       ├── android/            JNI، اطلاعات دستگاه، کنترل اکتیویتی
│       ├── ios/                اطلاعات دستگاه، پنجره، هپتیک
│       ├── windows/            پنجره، تم، اطلاعات دستگاه
│       ├── macos/              پنجره، تم، اطلاعات دستگاه
│       └── web/                user agent، viewport، تشخیص لمس
│
├── android-app/                پروژهٔ Android Studio. بدون سورس جاوا/کاتلین، بدون layout.
│   ├── settings.gradle.kts
│   ├── app/build.gradle.kts    به‌ازای هر ABI کراگو را اجرا و سپس .so را بسته‌بندی می‌کند
│   ├── app/src/main/
│   │   ├── AndroidManifest.xml به android.app.NativeActivity اشاره می‌کند
│   │   └── res/               رشته‌ها، تم، آیکون لانچر
│   ├── gradlew                 رَپر
│   └── proguard-rules.pro
│
├── resources/                  دادهٔ مشترک: strings_en.json، strings_es.json، colors.json
├── scripts/                    یک اسکریپت بیلد برای هر پلتفرم (+ dev.sh)
└── .github/workflows/ci.yml    شش job برای هر شش هدف به‌علاوهٔ بک‌اند
```

جهت وابستگی کاملاً یک‌طرفه است:

```
apps/example  ->  ui  ->  shared  <-  apps/server
       |           |        ^
       |           +-> platform/linux, /windows, /macos, /android, /ios, /web
       +-> resources
```

`shared` و `ui` هیچ‌وقت به crate پلتفرمی وابسته نیستند؛ برای همین `cargo check --workspace` روی
ماشینی که اصلاً SDK ندارد هم موفق می‌شود. `apps/server` فقط به `shared` وابسته است — هیچ‌وقت
به `ui` دست نمی‌زند — پس بک‌اند هیچ کد GUIای به باینری سرور نمی‌کشد.

## کار در IDE

پروژه عمداً مستقل از ادیتور است. هیچ پوشهٔ `.vscode/` یا `.idea/` کامیت نمی‌شود (به‌جز یک
کانفیگ VS Code که فقط برای «اجرا با F5» است، پایین‌تر را ببین)، پس پوشه را در هر ادیتوری که
دوست داری باز می‌کنی و ورک‌اسپیس خودش resolve می‌شود. سه فایل کامیت‌شده رفتار همهٔ ادیتورها را
یکسان می‌کنند:

- `rust-toolchain.toml` که `stable` را با کامپوننت‌های `rustfmt` و `clippy` پین می‌کند، تا
  تحلیل ادیتورت و ترمینالت از یک toolchain استفاده کنند — همان که CI استفاده می‌کند.
- `.editorconfig` که UTF-8، LF و تودرتویی ۴ فاصله (۲ برای TOML/YAML) را تعیین می‌کند. همهٔ
  ادیتورهای اصلی آن را نیتیو می‌خوانند.
- `.vscode/` که یک کانفیگ اجرای آماده و تنظیمات rust-analyzer دارد (پایین‌تر).

**ویرایش راست.** پوشهٔ ریشه را در ادیتوری با rust-analyzer (VS Code، Zed، Neovim) یا در
RustRover باز کن. ورک‌اسپیس، هر دوازده crate و همهٔ تست‌ها را بدون هیچ مرحلهٔ generation پیدا
می‌کند. یک نکته: کد پشت `#[cfg(target_os = "android")]` به‌طور پیش‌فرض type-check نمی‌شود، چون
rust-analyzer برای میزبانِ تو تحلیل می‌کند. برای پوشش آن، rust-analyzer را به یک هدف اندروید
اشاره بده — در VS Code، در `.vscode/settings.json` خودت:

```json
{
  "rust-analyzer.cargo.target": "aarch64-linux-android"
}
```

و برای کار دسکتاپ برش گردان. در هر حال، همان cross-compile در بخش دروازه‌های کیفیت است که
واقعاً آن کد را اثبات می‌کند.

**تفکیک feature در `ui`.** feature `egui` در `ui` به‌طور پیش‌فرض روشن است. اگر ادیتور آن را
با `--no-default-features` تحلیل کند، متدهای رندر از تحلیل حذف می‌شوند — این انتظار می‌رود و
`cargo check -p ui --no-default-features` چک صادقانه است.

**بسته‌بندی اندروید.** `android-app/` را در Android Studio باز کن تا sync، اجرا و APK را نصب
کنی (`./gradlew installDebug`). آن IDE برای بسته‌بندی است، نه محل نوشتن اپ: هیچ کاتلین یا
جاوا و هیچ XML layoutای برای ویرایش نیست، و راست در همان ادیتور راست نوشته می‌شود. Android
Studio خودش rust-analyzer ندارد، پس دو پنجره نگه دار — کد راست یک‌طرف، دستگاه و Logcat
طرف دیگر.

**اجرا از داخل VS Code.** با کانفیگ کامیت‌شده، F5 اپ را در حالت دیباگ اجرا می‌کند و
`dev.sh` را می‌خواهد بک‌اند هم بالا باشد. برای این کار یا:

- ترمینال را باز کن و `scripts/dev.sh` را اجرا کن، بعد F5 را بزن (کلاینت به آن وصل می‌شود)
  — یا
- تسک «Rustinaz: server + client» را از منوی Run Task اجرا کن که هر دو را با هم بالا می‌آورد.

## بیلد هر پلتفرم

```bash
scripts/build-all.sh --list      # این ماشین چه چیزهایی را می‌تواند بیلد کند
scripts/build-all.sh             # همه را بیلد کن
scripts/build-all.sh android web # فقط این‌ها
```

هرکدام به‌صورت جداگانه:

```bash
scripts/build-linux.sh [--release]
scripts/build-android.sh [--release] [--abi arm64-v8a] [--apk]
scripts/build-windows.sh [--release]     # به MinGW-w64 نیاز دارد، یا روی ویندوز اجرا کن
scripts/build-macos.sh [--release]       # فقط مک
scripts/build-ios.sh [--simulator]       # فقط مک
scripts/build-web.sh [--serve]           # به trunk نیاز دارد
scripts/dev.sh [--release] [--bind ADDR] # بک‌اند و کلاینت را با هم اجرا کن
```

### اندروید بدون Gradle

اگر فقط `.so` را می‌خواهی، کلاً Gradle را رد کن:

```bash
source scripts/android-env.sh
cargo build --package example-app --target aarch64-linux-android --release
# -> target/aarch64-linux-android/release/libexample_app.so
```

`scripts/android-env.sh` دلیل کارکردن این بدون ویرایش هیچ کانفیگی است. NDK از r23 به بعد
سیم‌لینک‌های خالی `<arch>-linux-android-clang` را حذف کرد، پس هم linker راست و هم کامپایلر C
که `cc-rs` لازم دارد باید صریح نام‌برده شوند. اسکریپت
`CARGO_TARGET_<TRIPLE>_LINKER`، `CC_<triple>`، `CXX_<triple>`، `AR_<triple>` و
`RANLIB_<triple>` را برای هر چهار ABI صادر می‌کند، و تسک `cargoBuild` در Gradle دقیقاً همان
مجموعه را export می‌کند.

## پکیج اندروید

`android-app/` یک پروژهٔ معمولی Android Studio است که تصادفاً هیچ سورس کدی در آن نیست.

- `AndroidManifest.xml` هم `android.app.NativeActivity` و هم
  `android.app.lib_name = "example_app"` را اعلام می‌کند. این تمام قرارداد با اندروید است:
  کتابخانه را لود کن و بگذار پنجره را در دست بگیرد.
- `app/build.gradle.kts` به‌ازای هر ABI یک‌بار `cargo build --target <triple>` را اجرا می‌کند،
  نتیجه‌ها را در `build/rustJniLibs/<abi>/` می‌چیند و می‌گذارد AGP آن‌ها را به‌عنوان `jniLibs`
  بسته‌بندی کند.
- پوشهٔ `res/` یک تم، دو رشته و یک آیکون لانچر دارد. همین تمام تنظیمات منبعی است که یک سطح
  نیتیو تمام‌صفحه می‌خواهد.

یک تنظیم غیرواضح: `packaging { jniLibs { keepDebugSymbols += "**/*.so" } }`. AGP به‌طور
پیش‌فرض یک stripper مخصوص ELF را روی هر کتابخانهٔ نیتیو بسته‌بندی‌شده اجرا می‌کند و روی یک
cdylib راست کراس‌کامپایل‌شده ممکن است فایل را اشتباه بفهمد و یک stub بریده برجایش بگذارد — یک
APK که نصب می‌شود و موقع اجرا کرش می‌کند. strip کار Cargo است (`[profile.release]` مقدار
`strip = "symbols"` را می‌گذارد)، چون Cargo همان toolchain LLVM را دارد که object را ساخته.
بیلدهای دیباگ نمادها را نگه می‌دارند تا دیباگر NDK بتواند کرش‌های نیتیو را symbolicate کند.

```bash
cd android-app
./gradlew assembleDebug                        # هر چهار ABI
./gradlew assembleDebug -PandroidAbi=arm64-v8a # یک ABI، بسیار سریع‌تر
./gradlew installDebug                         # بیلد و نصب روی دستگاه متصل
```

`adb install -r app/build/outputs/apk/debug/app-debug.apk`

منیفست `windowSoftInputMode="adjustResize"` را تنظیم می‌کند، پس کیبورد اندروید سطح را تغییر
اندازه می‌دهد به‌جای اینکه رویش را بپوشاند و ورودی متن egui اندازهٔ نو را دنبال می‌کند.

## تست و دروازه‌های کیفیت

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

چون feature `network` در `shared` به‌طور پیش‌فرض خاموش است، **`--all-features` اختیاری
نیست**: بدون آن، `DefaultNetworkService` واقعی هرگز کامپایل نمی‌شود و به‌جایش stub تست می‌شود.
نام کوتاه `cargo check-all` راه راحتی است که همه‌چیز را بخواهی:

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
```

هر crate تست واحد دارد. تست‌های کامپوننت یک فریم را داخل یک `egui::Context` رندر می‌کنند و
روی نتیجه assert می‌زنند، که بدون نیاز به display server هم پنیک‌ها و هم خطاهای هندسی را
می‌گیرد. تست‌های crateهای پلتفرم روی هر میزبانی اجرا می‌شوند چون هر crate پلتفرم برای
هدف‌های غیرنیتیو یک stub از هندل مخصوص سیستم‌عاملش کامپایل می‌کند.

crate `ui` از هر دو جهت چک می‌شود، چون تفکیک feature تمام ماجراست:

```bash
cargo check -p ui                       # با egui
cargo check -p ui --no-default-features # فقط منطق، بدون رندرر
```

`cargo test` روی میزبان، کد `#[cfg(target_os = "android")]` را **تمرین نمی‌کند**. تنها راه چک
کردن آن لایه، cross-compile کردنش است:

```bash
source scripts/android-env.sh
cargo build --package example-app --target aarch64-linux-android
```

## چه چیزی تست شده و چه چیزی نه

این را صریح می‌گوییم، چون قالبی که بی‌صدا بیش از توانش ادعا کند از یک قالب کوچک که راست
می‌گوید بدتر است.

**اینجا، از لینوکس، بیلد و اجرا شده:**

- `cargo check --workspace --all-targets --all-features` و
  `cargo test --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` و
  `cargo fmt --all --check`، هر دو پاک
- دسکتاپ لینوکس: `cargo run --package example-app` بیلد و اجرا می‌شود. همچنین با
  `--server http://127.0.0.1:8080` وقتی بک‌اند بالا بود، از طریق `scripts/dev.sh` اجرا شد.
- **بک‌اند**: `cargo run --package server` شروع شد و `curl` به `/api/health`، `/api/users`،
  `/api/users/search?q=…` و `POST /api/users` رسید، همه از قرارداد مشترک `api`. `scripts/dev.sh`
  بک‌اند و کلاینت را با هم بالا می‌آورد و با خروج، بک‌اند را می‌کشد.
- **اندروید**: یک `libexample_app.so` واقعی برای هر چهار ABI، هرکدام با `file` و `llvm-nm`
  تأیید شد که هم `ANativeActivity_onCreate` و هم `android_main` را export می‌کند. یک APK با
  Gradle ساخته و بررسی شد: شامل هر چهار کتابخانه، **صفر فایل `.class`** (منیفست واقعاً به
  `NativeActivity` خود فریم‌ورک اشاره می‌کند) و مقدار `android.app.lib_name` با کتابخانه
  هم‌خوان است.
- **هدف وب**: با Trunk از ابتدا تا انتها بیلد شد. `apps/example/dist/` یک `.wasm` واقعی،
  چسب wasm-bindgen آن و یک `index.html` با کانواس و اسکریپت شروع دارد.

**این‌جا قابلِ‌تأیید نیست و صادقانه همین‌طور علامت‌گذاری شده:**

- **iOS و مک** به مک و Xcode نیاز دارند. crateهای `crates/platform/ios` و
  `crates/platform/macos` به‌عنوان بخشی از چک ورک‌اسپیس کامپایل می‌شوند، اما این‌جا هیچ `.a`
  یا `.app`ی لینک نمی‌شود.
- **ویندوز** برای cross-compile به MinGW-w64 یا برای بیلد نیتیو به یک میزبان ویندوز نیاز دارد.
  این ماشین هیچ‌کدام را ندارد.
- **از این‌جا روی هیچ دستگاه یا امولاتور اندروید واقعی چیزی اجرا نشده.** APK سالم است و
  محتوایش بررسی شد، اما «درست بسته‌بندی می‌شود» و «درست اجرا می‌شود» دو ادعای متفاوت‌اند و
  فقط اولی تست شد.
- **دکمهٔ پنل بک‌اند این‌جا با دست انسان کلیک نشد.** مسیر شبکه‌اش `HttpNetworkService` است که
  رفت‌وبرگشت سوکت واقعی‌اش تست واحد دارد و plumbing تردِ کارگرش برای هر هدف کامپایل می‌شود،
  اما مسیر کلیک‌تا‌نتیجه از داخل GUI تمرین نشد. بیلد وب اصلاً به بک‌اند نمی‌رسد (مرورگر سوکت
  خام ندارد) و همین را روی صفحه می‌گوید.

ورک‌فلوی CI در `.github/workflows/ci.yml` هر شش هدف به‌علاوهٔ بک‌اند را با شش job پوشش می‌دهد و
همان‌جاست که هدف‌های تأییدنشده واقعاً اثبات می‌شوند.

## تغییر نام قالب به اپ خودت

ده‌ها جای مکانیکی است، اما تمرکز اصلی این‌هاست:

۱. `Cargo.toml` — `repository`، `authors`، `description` و `homepage` زیر `[workspace.package]`.
۲. `apps/example/Cargo.toml` — نام package یعنی `example-app` و `[lib] name` که باید
   `example_app` بماند مگر اینکه `rustLibName` در فایل Gradle و `android.app.lib_name` در
   منیفست را هم عوض کنی. هر سه باید هم‌خوان باشند.
۳. `android-app/app/build.gradle.kts` — `rustPackage` و `rustLibName`.
۴. `android-app/app/src/main/AndroidManifest.xml` — `android:name` و `android.app.lib_name`.
۵. `android-app/app/build.gradle.kts` — `namespace` و `applicationId`.
۶. `scripts/build-*.sh` و ورک‌فلوی CI — نام package یعنی `example-app`.
۷. `apps/server/` (اختیاری) — اگر نام package بک‌اند را عوض می‌کنی، `scripts/dev.sh` را
   (که `--package server` را بیلد و `$BIN_DIR/server` را اجرا می‌کند) و نام کوتاه `server` در
   `.cargo/config.toml` را هم به‌روز کن.

`libexample_app.so` همان نام کتابخانهٔ نیتیو اندروید است که در APK می‌نشیند. این نام از
`[lib] name` می‌آید، نه از نام package؛ برای همین این دو جدا تنظیم می‌شوند.

## مشارکت و گزارش آنچه یافتی

این قالبی است که برای تمرین‌دادن، شکستن و بهترکردن ساخته شده. اگر clone می‌کنی:

۱. دروازه‌های بخش [تست و دروازه‌های کیفیت](#تست-و-دروازههای-کیفیت) را روی **ماشین خودت**
   اجرا کن.
۲. همان هدفی را امتحان کن که واقعاً برایت مهم است — به‌خصوص هدف‌هایی که این‌جا فقط *کامپایل*
   شده‌اند: اندروید روی دستگاه واقعی، ویندوز، iOS و مک.
۳. وقتی چیزی شکست خورد، یک issue باز کن با دستور دقیق، متن کامل خطا، سیستم‌عاملت، toolchain
   (`rustc -V` و `cargo -V`) و برای اندروید نسخهٔ NDK.

یافته‌ها را گزارش کن به‌جای اینکه بی‌صدا دورشان بزنی — همین باعث می‌شود فهرست
«تست‌شده / تست‌نشده» بالا با گذر زمان کوتاه‌تر شود.

## نویسنده و راه ارتباطی

**Rustinaz** توسط **سینا خانزاده** طراحی و نگهداری می‌شود.

- رزومه / سایت: [sina-khanzadeh.ir](https://sina-khanzadeh.ir)
- ایمیل: [khanzadeh.1377@gmail.com](mailto:khanzadeh.1377@gmail.com)
- اینستاگرام / تلگرام / دیسکورد: `programmer_1998`
  — [اینستاگرام](https://instagram.com/programmer_1998) · [تلگرام](https://t.me/programmer_1998)
- گیت‌هاب: [@programmer-1998](https://github.com/programmer-1998)

## مجوز

MIT یا Apache-2.0، به انتخاب خودت.
