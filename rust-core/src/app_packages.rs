fn app_name_to_pkg(name: &str) -> String {
    let n = name.to_lowercase().trim().replace('-', " ").replace('_', " ");
    let pkg = match n.as_str() {
        // ── Google core ─────────────────────────────────────────────────
        "youtube"|"yt"|"you tube"                        => "com.google.android.youtube",
        "gmail"|"google mail"                            => "com.google.android.gm",
        "chrome"|"google chrome"                         => "com.android.chrome",
        "maps"|"google maps"|"gmap"|"gmaps"              => "com.google.android.apps.maps",
        "drive"|"google drive"                           => "com.google.android.apps.docs",
        "docs"|"google docs"                             => "com.google.android.apps.docs",
        "sheets"|"google sheets"|"spreadsheet"           => "com.google.android.apps.spreadsheets",
        "slides"|"google slides"|"presentation"          => "com.google.android.apps.docs",
        "photos"|"google photos"|"gphotos"               => "com.google.android.apps.photos",
        "calendar"|"google calendar"                     => "com.google.android.calendar",
        "meet"|"google meet"                             => "com.google.android.apps.meetings",
        "duo"|"google duo"                               => "com.google.android.apps.tachyon",
        "keep"|"google keep"|"notes"                     => "com.google.android.keep",
        "translate"|"google translate"                   => "com.google.android.apps.translate",
        "lens"|"google lens"                             => "com.google.ar.lens",
        "pay"|"google pay"|"gpay"                        => "com.google.android.apps.nbu.paisa.user",
        "classroom"|"google classroom"                   => "com.google.android.apps.classroom",
        "earth"|"google earth"                           => "com.google.earth",
        "fit"|"google fit"                               => "com.google.android.apps.fitness",
        "news"|"google news"                             => "com.google.android.apps.magazines",
        "play store"|"play"|"market"|"playstore"         => "com.android.vending",
        "play games"|"games"                             => "com.google.android.play.games",
        "play music"                                     => "com.google.android.music",
        "youtube music"|"yt music"|"ytmusic"             => "com.google.android.apps.youtube.music",
        "youtube kids"|"yt kids"                         => "com.google.android.apps.youtube.kids",
        "stadia"                                         => "com.google.stadia.android",
        "chrome beta"                                    => "com.chrome.beta",
        "chrome dev"                                     => "com.chrome.dev",
        "google assistant"|"assistant"                   => "com.google.android.googlequicksearchbox",
        "google search"|"search"                         => "com.google.android.googlequicksearchbox",
        "gemini"|"bard"                                  => "com.google.android.apps.bard",
        "google home"                                    => "com.google.android.apps.chromecast.app",
        "google one"                                     => "com.google.android.apps.subscriptions.red",
        "google tasks"|"tasks"                           => "com.google.android.apps.tasks",
        // ── System / Android ────────────────────────────────────────────
        "settings"                                       => "com.android.settings",
        "camera"|"cam"                                   => "com.android.camera2",
        "gallery"                                        => "com.android.gallery3d",
        "clock"|"alarm"|"timer"                          => "com.google.android.deskclock",
        "calculator"|"calc"                              => "com.google.android.calculator",
        "contacts"                                       => "com.google.android.contacts",
        "phone"|"dialer"|"call"                          => "com.google.android.dialer",
        "messages"|"sms"|"mms"|"android messages"        => "com.google.android.apps.messaging",
        "files"|"file manager"|"file explorer"           => "com.google.android.apps.nbu.files",
        "downloads"|"download manager"                   => "com.android.providers.downloads.ui",
        "browser"                                        => "com.android.browser",
        "music"|"media"|"player"                         => "com.google.android.music",
        "recorder"|"voice recorder"                      => "com.google.android.apps.recorder",
        "wallet"                                         => "com.google.android.apps.walletnfcrel",
        "accessibility"                                  => "com.google.android.marvin.talkback",
        "device health"|"battery saver"                  => "com.google.android.apps.turbo",
        "find my device"                                 => "com.google.android.apps.adm",
        "digital wellbeing"                              => "com.google.android.apps.wellbeing",
        "family link"                                    => "com.google.android.apps.kids.familylink",
        "android auto"                                   => "com.google.android.projection.gearhead",
        // ── Messaging / Social ───────────────────────────────────────────
        "whatsapp"|"wa"|"whats app"                      => "com.whatsapp",
        "whatsapp business"                              => "com.whatsapp.w4b",
        "telegram"|"tg"                                  => "org.telegram.messenger",
        "telegram x"                                     => "org.thunderdog.challegram",
        "instagram"|"ig"|"insta"                         => "com.instagram.android",
        "facebook"|"fb"                                  => "com.facebook.katana",
        "facebook messenger"|"messenger"                 => "com.facebook.orca",
        "facebook lite"                                  => "com.facebook.lite",
        "twitter"|"x"|"twitter x"                       => "com.twitter.android",
        "snapchat"|"snap"                                => "com.snapchat.android",
        "tiktok"|"tik tok"                               => "com.zhiliaoapp.musically",
        "discord"                                        => "com.discord",
        "reddit"                                         => "com.reddit.frontpage",
        "linkedin"                                       => "com.linkedin.android",
        "pinterest"                                      => "com.pinterest",
        "tumblr"                                         => "com.tumblr",
        "signal"                                         => "org.thoughtcrime.securesms",
        "viber"                                          => "com.viber.voip",
        "skype"                                          => "com.skype.raider",
        "line"                                           => "jp.naver.line.android",
        "kik"                                            => "kik.android",
        "wechat"|"we chat"                               => "com.tencent.mm",
        "imessage"                                       => "com.apple.MobileSMS",
        "imo"                                            => "com.imo.android.imoim",
        "hike"                                           => "com.bsb.hike",
        "clubhouse"                                      => "com.clubhouse.app",
        "mastodon"                                       => "org.joinmastodon.android",
        "threads"                                        => "com.instagram.barcelona",
        "bereal"                                         => "com.bereal.ft",
        // ── Entertainment ────────────────────────────────────────────────
        "netflix"                                        => "com.netflix.mediaclient",
        "spotify"                                        => "com.spotify.music",
        "amazon music"|"amazon prime music"              => "com.amazon.mp3",
        "prime video"|"amazon prime video"|"amazon video"=> "com.amazon.avod.thirdpartyclient",
        "disney plus"|"disney+"|"disneyplus"             => "com.disney.disneyplus",
        "hulu"                                           => "com.hulu.plus",
        "hbo max"|"max"                                  => "com.hbo.hbomax",
        "apple tv"|"apple tv+"                           => "com.apple.atve.amazon.appletv",
        "peacock"                                        => "com.peacocktv.peacockandroid",
        "paramount plus"|"paramount+"                    => "com.cbs.app",
        "twitch"                                         => "tv.twitch.android.app",
        "soundcloud"                                     => "com.soundcloud.android",
        "deezer"                                         => "deezer.android.app",
        "pandora"                                        => "com.pandora.android",
        "tidal"                                          => "com.aspiro.tidal",
        "shazam"                                         => "com.shazam.android",
        "audible"                                        => "com.audible.application",
        "plex"                                           => "com.plexapp.android",
        "vlc"                                            => "org.videolan.vlc",
        "kodi"                                           => "org.xbmc.kodi",
        "crunchyroll"                                    => "com.crunchyroll.crunchyroid",
        "mubi"                                           => "com.mubi",
        "vimeo"                                          => "com.vimeo.android.videoapp",
        "dailymotion"                                    => "com.dailymotion.dailymotion",
        "mixcloud"                                       => "com.mixcloud.android",
        // ── Shopping / Finance ───────────────────────────────────────────
        "amazon"|"amazon shopping"                       => "com.amazon.mShop.android.shopping",
        "ebay"                                           => "com.ebay.mobile",
        "flipkart"                                       => "com.flipkart.android",
        "myntra"                                         => "com.myntra.android",
        "meesho"                                         => "com.meesho.supply",
        "ajio"                                           => "com.ril.ajio",
        "nykaa"                                          => "com.nykaa.app",
        "paytm"                                          => "net.one97.paytm",
        "phonepe"                                        => "com.phonepe.app",

        "bhim"|"bhim upi"                                => "in.org.npci.upiapp",
        "paypal"                                         => "com.paypal.android.p2pmobile",
        "cash app"                                       => "com.squareup.cash",
        "venmo"                                          => "com.venmo",
        "wise"|"transferwise"                            => "com.transferwise.android",
        "coinbase"                                       => "com.coinbase.android",
        "binance"                                        => "com.binance.dev",
        "robinhood"                                      => "com.robinhood.android",
        "zerodha"|"kite"                                 => "com.zerodha.kite3",
        "groww"                                          => "com.nextbillion.groww",
        "upstox"                                         => "in.upstox.trading",
        // ── Navigation / Transport ───────────────────────────────────────
        "uber"                                           => "com.ubercab",
        "lyft"                                           => "me.lyft.android",
        "ola"|"ola cabs"                                 => "com.olacabs.customer",
        "rapido"                                         => "com.rapido.passenger",
        "grab"                                           => "com.grabtaxi.passenger",
        "waze"                                           => "com.waze",
        "here maps"|"here"                               => "com.here.app.maps",
        "maps me"|"mapsme"                               => "com.mapswithme.maps.pro",
        "citymapper"                                     => "com.citymapper.app.release",
        "moovit"                                         => "com.tranzmate",
        "sygic"                                          => "com.sygic.aura",
        "garmin"|"garmin connect"                        => "com.garmin.android.apps.connectmobile",
        "strava"                                         => "com.strava",
        // ── Productivity / Work ──────────────────────────────────────────
        "zoom"                                           => "us.zoom.videomeetings",
        "teams"|"microsoft teams"                        => "com.microsoft.teams",
        "slack"                                          => "com.Slack",
        "notion"                                         => "notion.id",
        "trello"                                         => "com.trello",
        "asana"                                          => "com.asana.app",
        "jira"                                           => "com.atlassian.android.jira.core",
        "monday"|"monday.com"                            => "com.monday.monday",
        "todoist"                                        => "com.todoist.android.Todoist",
        "any.do"|"any do"                                => "com.anydo",
        "ticktick"                                       => "com.ticktick.task",
        "microsoft office"|"office"                      => "com.microsoft.office.officehubrow",
        "word"|"microsoft word"                          => "com.microsoft.office.word",
        "excel"|"microsoft excel"                        => "com.microsoft.office.excel",
        "powerpoint"|"microsoft powerpoint"              => "com.microsoft.office.powerpoint",
        "outlook"|"microsoft outlook"                    => "com.microsoft.office.outlook",
        "onenote"|"microsoft onenote"                    => "com.microsoft.office.onenote",
        "onedrive"|"microsoft onedrive"                  => "com.microsoft.skydrive",
        "dropbox"                                        => "com.dropbox.android",
        "box"                                            => "com.box.android",
        "evernote"                                       => "com.evernote",
        "obsidian"                                       => "md.obsidian",
        "proton mail"|"protonmail"                       => "ch.protonmail.android",
        "hey email"                                      => "com.basecamp.hey",
        "spark email"|"spark"                            => "com.readdle.spark",
        "canva"                                          => "com.canva.editor",
        "adobe express"                                  => "com.adobe.spark.post",
        "adobe acrobat"|"acrobat"                        => "com.adobe.reader",
        "adobe lightroom"|"lightroom"                    => "com.adobe.lrmobile",
        "snapseed"                                       => "com.niksoftware.snapseed",
        "vsco"                                           => "com.vsco.cam",
        "remini"                                         => "com.bigwinepot.nwc.international",
        "1password"                                      => "com.agilebits.onepassword",
        "bitwarden"                                      => "com.x8bit.bitwarden",
        "lastpass"                                       => "com.lastpass.lpandroid",
        "dashlane"                                       => "com.dashlane",
        "nordvpn"                                        => "com.nordvpn.android",
        "expressvpn"                                     => "com.expressvpn.vpn",
        "proton vpn"|"protonvpn"                         => "ch.protonvpn.android",
        // ── Food / Delivery ──────────────────────────────────────────────
        "swiggy"                                         => "in.swiggy.android",
        "zomato"                                         => "com.application.zomato",
        "uber eats"|"ubereats"                           => "com.ubercab.eats",
        "doordash"                                       => "com.dd.doordash",
        "instacart"                                      => "com.instacart.client",
        "grubhub"                                        => "com.grubhub.android",
        "dunzo"                                          => "com.dunzo.user",
        "blinkit"|"grofers"                              => "com.grofers.customerapp",
        "bigbasket"                                      => "com.bigbasket",
        "zepto"                                          => "com.zepto.app",
        // ── Health / Fitness ─────────────────────────────────────────────
        "samsung health"|"s health"                      => "com.sec.android.app.shealth",
        "fitbit"                                         => "com.fitbit.FitbitMobile",
        "myfitnesspal"                                   => "com.myfitnesspal.android",
        "lifesum"                                        => "com.sillens.shapeupclub",
        "nike run club"|"nike running"|"nike run"        => "com.nike.plusgps",
        "adidas running"|"runtastic"                     => "com.runtastic.android",
        "runkeeper"                                      => "com.fitnesskeeper.runkeeper.pro",
        "headspace"                                      => "com.getsomeheadspace.android",
        "calm"                                           => "com.calm.android",
        "sleep cycle"                                    => "com.northcube.sleepcycle",
        "period tracker"|"flo"|"flo health"              => "org.iggymedia.periodtracker",
        "blood pressure"|"bp monitor"                    => "com.qardio.android",
        // ── News / Reading ───────────────────────────────────────────────
        "inshorts"                                       => "com.nis.app",
        "flipboard"                                      => "flipboard.app",
        "feedly"                                         => "com.devhd.feedly",
        "pocket"                                         => "com.ideashower.readitlater.pro",
        "medium"                                         => "com.medium.reader",
        "kindle"                                         => "com.amazon.kindle",
        "kobo"                                           => "com.kobobooks.android",
        "scribd"                                         => "com.scribd.app.reader0",
        "duolingo"                                       => "com.duolingo",
        "babbel"                                         => "com.babbel.mobile.android.en",
        // ── Gaming ───────────────────────────────────────────────────────
        "pubg"|"bgmi"|"battlegrounds mobile"             => "com.pubg.imobile",
        "free fire"|"freefire"|"garena free fire"        => "com.dts.freefireth",
        "minecraft"                                      => "com.mojang.minecraftpe",
        "roblox"                                         => "com.roblox.client",
        "candy crush"|"candy crush saga"                 => "com.king.candycrushsaga",
        "among us"                                       => "com.innersloth.spacemafia",
        "clash of clans"|"coc"                           => "com.supercell.clashofclans",
        "clash royale"                                   => "com.supercell.clashroyale",
        "mobile legends"|"mlbb"                          => "com.mobile.legends",
        "pokemon go"                                     => "com.nianticlabs.pokemongo",
        "ludo king"                                      => "com.ludo.king",
        "8 ball pool"                                    => "com.miniclip.eightballpool",
        "chess"|"chess.com"                              => "com.chess",
        "steam"                                          => "com.valvesoftware.android.steam.community",
        // ── Travel ───────────────────────────────────────────────────────
        "airbnb"                                         => "com.airbnb.android",
        "booking.com"|"booking"                          => "com.booking",
        "tripadvisor"                                    => "com.tripadvisor.tripadvisor",
        "expedia"                                        => "com.expedia.bookings",
        "makemytrip"|"mmt"                               => "com.makemytrip",
        "goibibo"                                        => "com.goibibo",
        "cleartrip"                                      => "com.cleartrip.android",
        "ixigo"                                          => "com.ixigo.train.ixitrain",
        "irctc"|"irctc rail connect"                     => "cris.org.in.prs.ima",
        "redbus"                                         => "in.redbus.android",
        "trainman"                                       => "com.trainman.app",
        // ── Developer / Utility ──────────────────────────────────────────
        "termux"                                         => "com.termux",
        "adb"|"adb wifi"                                 => "com.ttxapps.wifiadb",
        "ssh"|"juicessh"|"termius"                       => "com.server.auditor.ssh.client",
        "github"                                         => "com.github.android",
        "gitlab"                                         => "com.gitlab.android",
        "stackoverflow"                                  => "com.stackexchange.stackoverflow",
        "chrome remote desktop"                          => "com.google.chromeremotedesktop",
        "anydesk"                                        => "com.anydesk.anydeskandroid",
        "teamviewer"                                     => "com.teamviewer.teamviewer.market.mobile",
        "winrar"                                         => "com.rarlab",
        "cx file explorer"                               => "com.cxinventor.file.explorer",
        "es file explorer"                               => "com.estrongs.android.pop",
        "solid explorer"                                 => "pl.solidexplorer2",
        "mixplorer"                                      => "com.mixplorer",
        "qr scanner"|"qr code"                           => "me.scan.android.scanner",
        "barcode scanner"                                => "com.google.zxing.client.android",
        "cpu z"                                          => "com.cpuid.cpu_z",
        "antutu"                                         => "com.antutu.ABenchMark",
        "wifi analyzer"|"wifi analyser"                  => "com.farproc.wifi.analyzer",
        "gsam battery"                                   => "com.gsamlabs.bbm",
        "accubattery"                                    => "com.digibites.accubattery",
        "magisk"                                         => "io.github.huskydg.magisk",
        "shizuku"                                        => "moe.shizuku.privileged.api",
        "obtainium"                                      => "dev.imranr.obtainium",
        // ── Samsung specific ─────────────────────────────────────────────
        "samsung camera"                                 => "com.sec.android.app.camera",
        "samsung gallery"                                => "com.sec.android.gallery3d",
        "samsung internet"                               => "com.sec.android.app.sbrowser",
        "samsung pay"                                    => "com.samsung.android.spay",
        "samsung notes"|"s note"                         => "com.samsung.android.app.notes",
        "samsung bixby"|"bixby"                          => "com.samsung.android.bixby.agent",
        "samsung store"|"galaxy store"                   => "com.sec.android.app.samsungapps",

        "samsung music"                                  => "com.sec.android.app.music",
        "samsung clock"                                  => "com.sec.android.app.clockpackage",
        "dex"|"samsung dex"                              => "com.samsung.android.desktopmode.uiservice",
        // ── Xiaomi specific ──────────────────────────────────────────────
        "miui camera"|"mi camera"                        => "com.android.camera",
        "mi gallery"|"miui gallery"                      => "com.miui.gallery",
        "mi browser"|"miui browser"                      => "com.mi.globalbrowser",
        "mi store"                                       => "com.xiaomi.mihome",
        "mi music"                                       => "com.miui.player",
        "mi video"                                       => "com.miui.videoplayer",
        "mi home"|"xiaomi home"                          => "com.xiaomi.smarthome",
        "mi pay"                                         => "com.mipay.wallet.in",
        "mi calculator"                                  => "com.miui.calculator",
        "mi cleaner"|"miui cleaner"                      => "com.miui.cleanmaster",
        "miui themes"                                    => "com.mi.android.globalminusscreen",
        // ── Other popular ────────────────────────────────────────────────
        "brave browser"|"brave"                          => "com.brave.browser",
        "firefox"|"firefox browser"                      => "org.mozilla.firefox",
        "opera"|"opera browser"                          => "com.opera.browser",
        "duckduckgo"|"ddg"                               => "com.duckduckgo.mobile.android",
        "tor browser"                                    => "org.torproject.torbrowser",
        "edge"|"microsoft edge"                          => "com.microsoft.emmx",
        "vivaldi"                                        => "com.vivaldi.browser",
        "via browser"|"via"                              => "mark.via.gp",
        "kiwi browser"                                   => "com.kiwibrowser.browser",
        "cx browser"                                     => "com.cxinventor.browse",
        "mx player"                                      => "com.mxtech.videoplayer.ad",
        "video player"                                   => "com.mxtech.videoplayer.ad",
        "poweramp"                                       => "com.maxmpz.audioplayer",
        "musicolet"                                      => "in.krosbits.musicolet",
        "neutron music"                                  => "com.neutroncode.mp",
        "anki"                                           => "com.ichi2.anki",
        "wikipedia"                                      => "org.wikipedia",
        "wolfram alpha"                                  => "com.wolfram.android.alpha",
        "moon reader"|"moon+ reader"                     => "com.flyersoft.moonreader",
        "tasker"                                         => "net.dinglisch.android.taskerm",
        "automate"                                       => "com.llamalab.automate",
        "macrodroid"                                     => "com.arlosoft.macrodroid",
        "airtable"                                       => "com.formagrid.airtable",
        "zapier"                                         => "com.zapier.android",
        "ifttt"                                          => "com.ifttt.ifttt",
        "shortcuts"|"shortcut"                           => "com.google.android.apps.shortcuts",
        "chatgpt"|"chat gpt"                             => "com.openai.chatgpt",
        "claude"                                         => "com.anthropic.claude",
        "perplexity"                                     => "ai.perplexity.app.android",

        "copilot"|"microsoft copilot"                    => "com.microsoft.copilot",
        "grok"                                           => "com.x.android",
        _                                                => &n,
    };
    if pkg != n.as_str() { pkg.to_string() } else { name.trim().to_string() }
}

fn route_openclaw_v3(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // \u{2500}\u{2500} DSL Script execution
        ("POST", "/dsl/run") => {
            let macro_id = extract_json_str(body, "macro_id").unwrap_or_else(gen_id);
            let script   = extract_json_str(body, "script").unwrap_or_default();
            let log = execute_dsl_script(&mut STATE.lock().unwrap(), &macro_id, &script);
            Some(format!(r#"{{"ok":true,"log":[{}]}}"#,
                log.iter().map(|l| format!(r#""{}""#, esc(l))).collect::<Vec<_>>().join(",")))
        }

        // \u{2500}\u{2500} Reactive subscriptions
        ("GET",  "/rx/subscriptions") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.rx_subscriptions.iter().map(|sub|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"fired":{}}}"#,
                    esc(&sub.id), esc(&sub.name), sub.enabled, sub.fired_count)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/rx/subscribe") => {
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            let name   = extract_json_str(body, "name").unwrap_or_default();
            let kinds_str = extract_json_str(body, "event_kinds").unwrap_or_default();
            let event_kinds: Vec<String> = kinds_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let target_macro = extract_json_str(body, "target_macro").unwrap_or_default();
            let debounce_ms  = extract_json_num(body, "debounce_ms").unwrap_or(0.0) as u128;
            let throttle_ms  = extract_json_num(body, "throttle_ms").unwrap_or(0.0) as u128;
            let mut operators = Vec::new();
            if debounce_ms > 0 { operators.push(RxOperator::Debounce(debounce_ms)); }
            if throttle_ms > 0 { operators.push(RxOperator::Throttle(throttle_ms)); }
            if body.contains(r#""distinct":true"#) { operators.push(RxOperator::Distinct); }
            let sub = RxSubscription {
                id: id.clone(), name, event_kinds, operators, target_macro, enabled: true,
                fired_count: 0, last_fired: 0, debounce_last: 0, throttle_last: 0,
                take_count: 0, skip_count: 0, last_value: String::new(), buffer: Vec::new(),
            };
            STATE.lock().unwrap().rx_subscriptions.push(sub);
            Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
        }
        ("POST", "/rx/event") => {
            let kind = extract_json_str(body, "kind").unwrap_or_default();
            let data = extract_json_str(body, "data").unwrap_or_default();
            let event = RxEvent { kind: kind.clone(), data, ts: now_ms(), source: "api".to_string() };
            let mut s = STATE.lock().unwrap();
            let subs: Vec<RxSubscription> = s.rx_subscriptions.iter().cloned().collect();
            for mut sub in subs {
                if !sub.enabled { continue; }
                if let Some(_payload) = rx_process_event(&mut sub, &event, &s) {
                    let target = sub.target_macro.clone();
                    chain_macro(&mut s, &target);
                    if let Some(rs) = s.rx_subscriptions.iter_mut().find(|r| r.id == sub.id) {
                        rs.fired_count += 1; rs.last_fired = now_ms();
                    }
                }
            }
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} State machines
        ("GET",  "/fsm/machines")   => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.state_machines.iter().map(|m|
                format!(r#"{{"id":"{}","name":"{}","state":"{}","enabled":{}}}"#,
                    esc(&m.id), esc(&m.name), esc(&m.current_state), m.enabled)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/fsm/event")      => {
            let machine_id = extract_json_str(body, "machine_id").unwrap_or_default();
            let event_kind = extract_json_str(body, "event").unwrap_or_default();
            fsm_process_event(&mut STATE.lock().unwrap(), &machine_id, &event_kind);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Context zones
        ("GET",  "/zones")          => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.context_zones.iter().map(|z|
                format!(r#"{{"id":"{}","name":"{}","active":{},"profile":"{}"}}"#,
                    esc(&z.id), esc(&z.name), z.currently_active, esc(&z.activate_profile))
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }

        // \u{2500}\u{2500} Bundle export/import
        ("GET",  "/bundle/export")  => {
            let tag = path.find("tag=").map(|i| &path[i+4..]).map(|s| s.split('&').next().unwrap_or(""));
            Some(export_bundle(&STATE.lock().unwrap(), tag))
        }
        ("POST", "/bundle/import")  => {
            import_macros_json(&mut STATE.lock().unwrap(), body);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Channel messaging
        ("POST", "/channel/post")   => {
            let ch  = extract_json_str(body, "channel").unwrap_or_default();
            let msg = extract_json_str(body, "message").unwrap_or_default();
            channel_post(&mut STATE.lock().unwrap(), &ch, &msg);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Battery-aware scheduling
        ("POST", "/battery/defer")  => {
            let macro_id = extract_json_str(body, "macro_id").unwrap_or_default();
            let min_pct  = extract_json_num(body, "min_pct").unwrap_or(20.0) as i32;
            defer_until_charged(&mut STATE.lock().unwrap(), &macro_id, min_pct);
            Some(format!(r#"{{"ok":true,"deferred":"{}","min_pct":{}}}"#, esc(&macro_id), min_pct))
        }

        // ── v43: Natural-language automation shortcuts ───────────────────────
        // Simple HTTP API that maps plain intents → macro objects.
        // Called by KiraTools.runTool("if_then", ...) and by AI chat.

        // POST /auto/if_then {"if":"battery < 20","then":"notify me low battery"}
        ("POST", "/auto/if_then") => {
            let cond_str   = extract_json_str(body, "if").unwrap_or_default();
            let action_str = extract_json_str(body, "then").unwrap_or_default();
            let id         = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if cond_str.is_empty() || action_str.is_empty() {
                return Some(r#"{"error":"need if and then fields"}"#.to_string());
            }
            let (tkind, tval) = parse_nl_condition(&cond_str);
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"if {} then {}","enabled":true,"triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&cond_str), esc(&action_str),
                esc(&tkind), esc(&tval), esc(&action_str)
            ));
            let mid = m.id.clone(); let mname = m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","trigger":"{}","val":"{}"}}"#,
                esc(&mid), esc(&mname), esc(&tkind), esc(&tval)))
        }

        // POST /auto/watch_app {"app":"youtube","action":"log I opened YouTube"}
        ("POST", "/auto/watch_app") => {
            let app    = extract_json_str(body, "app").unwrap_or_default();
            let action = extract_json_str(body, "action").unwrap_or_default();
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if app.is_empty() { return Some(r#"{"error":"need app"}"#.to_string()); }
            let pkg = app_name_to_pkg(&app);
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"when {} opens","enabled":true,"triggers":[{{"kind":"app_opened","config":{{"package":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&app), esc(&pkg), esc(&action)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","app":"{}","pkg":"{}"}}"#,
                esc(&mid), esc(&app), esc(&pkg)))
        }

        // POST /auto/repeat {"task":"check battery","every_minutes":30}
        ("POST", "/auto/repeat") => {
            let task    = extract_json_str(body, "task").unwrap_or_default();
            let minutes = extract_json_num(body, "every_minutes").unwrap_or(30.0) as u64;
            let id      = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if task.is_empty() { return Some(r#"{"error":"need task"}"#.to_string()); }
            let interval_ms = minutes * 60_000;
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"every {}min: {}","enabled":true,"triggers":[{{"kind":"interval","config":{{"interval_ms":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), minutes, esc(&task), interval_ms, esc(&task)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","task":"{}","every_minutes":{}}}"#,
                esc(&mid), esc(&task), minutes))
        }

        // POST /auto/on_notif {"keyword":"OTP","action":"read aloud","app":""}
        ("POST", "/auto/on_notif") => {
            let keyword = extract_json_str(body, "keyword").unwrap_or_default();
            let action  = extract_json_str(body, "action").unwrap_or_default();
            let app     = extract_json_str(body, "app").unwrap_or_default();
            let id      = extract_json_str(body, "id").unwrap_or_else(gen_id);
            let tkind = if app.is_empty() { "keyword_notif" } else { "app_notif" };
            let tval  = if app.is_empty() { keyword.clone() } else { app_name_to_pkg(&app) };
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"on notif '{}': {}","enabled":true,"tags":["notification"],"triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&keyword), esc(&action),
                esc(tkind), esc(&tval), esc(&action)
            ));
            let mid = m.id.clone(); let mname = m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","keyword":"{}"}}"#,
                esc(&mid), esc(&mname), esc(&keyword)))
        }

        // POST /auto/on_time {"time":"07:30","action":"good morning","days":"daily"}
        ("POST", "/auto/on_time") => {
            let time   = extract_json_str(body, "time").unwrap_or_else(|| "08:00".to_string());
            let action = extract_json_str(body, "action").unwrap_or_default();
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if action.is_empty() { return Some(r#"{"error":"need action"}"#.to_string()); }
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"at {}: {}","enabled":true,"tags":["scheduled"],"triggers":[{{"kind":"time_daily","config":{{"time":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&time), esc(&action), esc(&time), esc(&action)
            ));
            let mid = m.id.clone();
            let mut s = STATE.lock().unwrap();
            schedule_macro_daily(&mut s, &id, &time);
            s.macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","time":"{}","action":"{}"}}"#,
                esc(&mid), esc(&time), esc(&action)))
        }

        // POST /auto/on_charge {"action":"run backup","state":"plugged"}
        ("POST", "/auto/on_charge") => {
            let action = extract_json_str(body, "action").unwrap_or_default();
            let state  = extract_json_str(body, "state").unwrap_or_else(|| "plugged".to_string());
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if action.is_empty() { return Some(r#"{"error":"need action"}"#.to_string()); }
            let tkind = if state == "unplugged" { "power_disconnected" } else { "power_connected" };
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"on {}: {}","enabled":true,"triggers":[{{"kind":"{}","config":{{}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&state), esc(&action), esc(tkind), esc(&action)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","state":"{}","action":"{}"}}"#,
                esc(&mid), esc(&state), esc(&action)))
        }

        // GET /auto/list  — friendly summary of all automations
        ("GET", "/auto/list") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macros.iter().map(|m| {
                let tsum = m.triggers.first().map(|t| t.kind.to_str().to_string()).unwrap_or_default();
                let asum = m.actions.first()
                    .map(|a| a.params.get("message").cloned().unwrap_or_else(|| a.kind.to_str().to_string()))
                    .unwrap_or_default();
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"runs":{},"trigger":"{}","action":"{}","tags":[{}]}}"#,
                    esc(&m.id), esc(&m.name), m.enabled, m.run_count,
                    esc(&tsum), esc(&asum[..asum.len().min(60)]),
                    m.tags.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","))
            }).collect();
            Some(format!(r#"{{"ok":true,"count":{},"automations":[{}]}}"#, items.len(), items.join(",")))
        }

        // POST /auto/enable {"id":"...","enabled":true}
        ("POST", "/auto/enable") => {
            let id  = extract_json_str(body, "id").unwrap_or_default();
            let ena = !body.contains(r#""enabled":false"#);
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
                m.enabled = ena;
                Some(format!(r#"{{"ok":true,"id":"{}","enabled":{}}}"#, esc(&id), ena))
            } else {
                Some(format!(r#"{{"error":"automation '{}' not found"}}"#, esc(&id)))
            }
        }

        // DELETE /auto/:id
        ("DELETE", auto_path) if auto_path.starts_with("/auto/") => {
            let id = auto_path.trim_start_matches("/auto/");
            let mut s = STATE.lock().unwrap();
            let before = s.macros.len();
            s.macros.retain(|m| m.id != id);
            Some(format!(r#"{{"ok":true,"removed":{}}}"#, before - s.macros.len()))
        }

        // OpenClaw v3: Advanced automation features

        // GET /auto/templates
        ("GET", "/auto/templates") => {
            let templates = vec![
                ("morning_routine",    "Morning routine",      "time_daily",    "07:00"),
                ("low_battery_alert",  "Low battery alert",    "battery_low",   "20"),
                ("youtube_opened",     "Log YouTube usage",    "app_opened",    "com.google.android.youtube"),
                ("screen_off_silence", "Silence on screen off","screen_off",    ""),
                ("wifi_greeter",       "WiFi connected",       "wifi_changed",  "connected"),
                ("morning_briefing",   "Morning briefing",     "time_daily",    "06:30"),
                ("night_mode",         "Night mode at 22:00",  "time_daily",    "22:00"),
                ("shake_screenshot",   "Shake to screenshot",  "shake",         ""),
                ("sms_reader",         "Read incoming SMS",    "sms_received",  ""),
                ("call_logger",        "Log missed calls",     "call_missed",   ""),
                ("bt_audio",           "BT connected",         "bt_connected",  ""),
                ("charge_done",        "Charge complete",      "battery_low",   "95"),
            ];
            let items: Vec<String> = templates.iter().map(|(id, name, tkind, tval)|
                format!(r#"{{"id":"{}","name":"{}","trigger_kind":"{}","trigger_val":"{}"}}"#,
                    id, name, tkind, tval)
            ).collect();
            Some(format!(r#"{{"ok":true,"count":{},"templates":[{}]}}"#,
                items.len(), items.join(",")))
        }

        // POST /auto/from_template {"template_id":"morning_routine","action":"...","time":"07:30"}
        ("POST", "/auto/from_template") => {
            let tpl_id      = extract_json_str(body, "template_id").unwrap_or_default();
            let custom_act  = extract_json_str(body, "action").unwrap_or_default();
            let time_ov     = extract_json_str(body, "time").unwrap_or_default();
            let macro_id    = extract_json_str(body, "id").unwrap_or_else(|| format!("tpl_{}", gen_id()));

            let (tkind, tval, default_act) = match tpl_id.as_str() {
                "morning_routine"    => ("time_daily",   "07:00",  "good morning, give me today summary"),
                "low_battery_alert"  => ("battery_low",  "20",     "my battery is low, please charge"),
                "youtube_opened"     => ("app_opened",   "com.google.android.youtube", "log YouTube session started"),
                "screen_off_silence" => ("screen_off",   "",       "mute volume"),
                "wifi_greeter"       => ("wifi_changed", "connected","WiFi connected, checking updates"),
                "morning_briefing"   => ("time_daily",   "06:30",  "give me today morning briefing"),
                "night_mode"         => ("time_daily",   "22:00",  "set screen brightness to minimum"),
                "shake_screenshot"   => ("shake",        "",       "take a screenshot"),
                "sms_reader"         => ("sms_received", "",       "read the latest SMS message aloud"),
                "call_logger"        => ("call_missed",  "",       "log missed call received"),
                "bt_audio"           => ("bt_connected", "",       "bluetooth audio device connected"),
                "charge_done"        => ("battery_low",  "95",     "battery fully charged"),
                _                   => ("manual",        "",       "run automation"),
            };

            let tval_final = if !time_ov.is_empty() { time_ov.as_str() } else { tval };
            let act_final  = if !custom_act.is_empty() { custom_act.as_str() } else { default_act };

            let m = parse_macro_from_json(&format!(
                concat!(r#"{{"id":"{}","name":"[{}] {}","enabled":true,"tags":["template","{}"],"#,
                        r#""triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"#,
                        r#""conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#),
                esc(&macro_id), esc(&tpl_id), esc(act_final),
                esc(&tpl_id), esc(tkind), esc(tval_final), esc(act_final)
            ));
            let mid  = m.id.clone();
            let mname= m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","trigger":"{}","val":"{}"}}"#,
                esc(&mid), esc(&mname), esc(tkind), esc(tval_final)))
        }

        // POST /auto/scene {"name":"work mode","actions":["mute notifications","open calendar"]}
        ("POST", "/auto/scene") => {
            let name    = extract_json_str(body, "name").unwrap_or_default();
            let scene_id= extract_json_str(body, "id")
                .unwrap_or_else(|| format!("scene_{}", name.to_lowercase().replace(' ', "_")));
            if name.is_empty() { return Some(r#"{"error":"need name"}"#.to_string()); }
            // Extract actions array content
            let acts: Vec<String> = {
                let key = "\"actions\":[";
                if let Some(start) = body.find(key) {
                    let after = &body[start + key.len()..];
                    if let Some(end) = after.find(']') {
                        after[..end].split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    } else { vec![] }
                } else { vec![] }
            };
            let combined = acts.join(", then ");
            let m = parse_macro_from_json(&format!(
                concat!(r#"{{"id":"{}","name":"scene: {}","enabled":true,"tags":["scene"],"#,
                        r#""triggers":[{{"kind":"manual","config":{{}}}}],"#,
                        r#""conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#),
                esc(&scene_id), esc(&name), esc(&combined)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","steps":{}}}"#,
                esc(&mid), esc(&name), acts.len()))
        }

        // POST /auto/run_now {"id":"macro_id"} — trigger immediately
        ("POST", "/auto/run_now") => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            if id.is_empty() { return Some(r#"{"error":"need id"}"#.to_string()); }
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter().find(|m| m.id == id).cloned() {
                let name = m.name.clone();
                let (steps, _ok) = execute_macro_actions(&mut s, &id, &m.actions);
                Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","steps":{}}}"#,
                    esc(&id), esc(&name), steps))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&id)))
            }
        }

        // POST /auto/pause {"id":"macro_id","resume_after_minutes":30}
        ("POST", "/auto/pause") => {
            let id      = extract_json_str(body, "id").unwrap_or_default();
            let minutes = extract_json_num(body, "resume_after_minutes").unwrap_or(60.0) as u64;
            if id.is_empty() { return Some(r#"{"error":"need id"}"#.to_string()); }
            let resume_ms = now_ms() + (minutes as u128) * 60_000;
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
                m.enabled = false;
                let resume_trigger = Trigger {
                    id:           format!("resume_{}", id),
                    trigger_type: "time".to_string(),
                    value:        resume_ms.to_string(),
                    action:       format!("enable_macro:{}", id),
                    fired:        false,
                    repeat:       false,
                };
                s.triggers.push(resume_trigger);
                Some(format!(r#"{{"ok":true,"id":"{}","paused_minutes":{}}}"#,
                    esc(&id), minutes))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&id)))
            }
        }

        // GET /auto/history — last 50 runs
        ("GET", "/auto/history") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macro_run_log.iter().rev().take(50)
                .map(|r| format!(
                    r#"{{"id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"ms":{},"ts":{}}}"#,
                    esc(&r.macro_id), esc(&r.macro_name), esc(&r.trigger),
                    r.success, r.steps_run, r.duration_ms, r.ts))
                .collect();
            Some(format!(r#"{{"ok":true,"count":{},"history":[{}]}}"#,
                items.len(), items.join(",")))
        }

        // GET /auto/stats
        ("GET", "/auto/stats") => {
            let s = STATE.lock().unwrap();
            let enabled  = s.macros.iter().filter(|m| m.enabled).count();
            let total_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let success  = s.macro_run_log.iter().filter(|r| r.success).count();
            let failed   = s.macro_run_log.iter().filter(|r| !r.success).count();
            Some(format!(
                r#"{{"total":{},"enabled":{},"disabled":{},"total_runs":{},"success":{},"failed":{}}}"#,
                s.macros.len(), enabled, s.macros.len()-enabled,
                total_runs, success, failed))
        }

        // POST /auto/clone {"id":"src","new_id":"dst","new_name":"Copy of ..."}
        ("POST", "/auto/clone") => {
            let src     = extract_json_str(body, "id").unwrap_or_default();
            let new_id  = extract_json_str(body, "new_id").unwrap_or_else(|| format!("clone_{}", gen_id()));
            let new_nm  = extract_json_str(body, "new_name").unwrap_or_default();
            let mut s   = STATE.lock().unwrap();
            if let Some(original) = s.macros.iter().find(|m| m.id == src).cloned() {
                let mut c    = original.clone();
                c.id         = new_id.clone();
                c.name       = if new_nm.is_empty() { format!("{} (copy)", original.name) } else { new_nm };
                c.run_count  = 0;
                let cname    = c.name.clone();
                s.macros.push(c);
                Some(format!(r#"{{"ok":true,"id":"{}","name":"{}"}}"#, esc(&new_id), esc(&cname)))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&src)))
            }
        }

        // POST /auto/batch_enable {"ids":["a","b"],"enabled":true}
        ("POST", "/auto/batch_enable") => {
            let enabled = !body.contains("\"enabled\":false");
            let ids_raw = body.find("\"ids\":[")
                .map(|i| { let after = &body[i+7..]; after[..after.find(']').unwrap_or(0)].to_string() })
                .unwrap_or_default();
            let ids: Vec<&str> = ids_raw.split(',')
                .map(|s| s.trim().trim_matches('"'))
                .filter(|s| !s.is_empty())
                .collect();
            let mut s = STATE.lock().unwrap();
            let mut count = 0usize;
            for m in s.macros.iter_mut() {
                if ids.contains(&m.id.as_str()) { m.enabled = enabled; count += 1; }
            }
            Some(format!(r#"{{"ok":true,"updated":{}}}"#, count))
        }

        _ => None,
    }
}

// / \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Roboru / E-Robot / Automate Engine
// Inspired by: LlamaLab Automate (flowchart), E-Robot (170+ events, 150+ actions),
// Robot Framework (keyword-driven RPA), UiPath (intelligent automation)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Flowchart block types (Automate-style visual programming)
#[derive(Clone, Debug)]
enum FlowBlockKind {
    Start,
    Stop,
    Action,       // execute a MacroAction
    Decision,     // if/else branch
    Loop,         // for/while loop
    Wait,         // delay
    Fork,         // parallel execution branches
    Join,         // wait for all parallel branches
    SubFlow,      // call another flow by id
    Catch,        // error handler
    Log,          // debug logging block
}

/// A node in the visual flowchart
#[derive(Clone)]
struct FlowBlock {
    id:           String,
    kind:         FlowBlockKind,
    label:        String,
    // Connections: next block ids
    next:         Vec<String>,   // [0]=true branch, [1]=false branch for Decision
    // Payload
    action:       Option<MacroAction>,
    condition:    Option<MacroCondition>,
    loop_count:   u32,
    loop_var:     String,   // variable to increment each loop
    sub_flow_id:  String,   // for SubFlow blocks
    // Retry config (E-Robot pattern)
    retry_count:  u32,
    retry_delay_ms: u64,
}

/// A complete visual flow (like Automate's flowchart)
#[derive(Clone)]
struct AutoFlow {
    id:          String,
    name:        String,
    description: String,
    enabled:     bool,
    start_block: String,
    blocks:      HashMap<String, FlowBlock>,
    created_ms:  u128,
    run_count:   u64,
    last_run_ms: u128,
    tags:        Vec<String>,
}

/// Keyword definition (Robot Framework pattern)
/// A named reusable action sequence
#[derive(Clone)]
struct Keyword {
    name:        String,  // e.g. "Open And Login YouTube"
    description: String,
    steps:       Vec<MacroAction>,
    args:        Vec<String>,  // parameter names
    returns:     String,  // variable name to store result
}

/// Hyper-automation pipeline step (UiPath/Comidor pattern)
/// Combines BPM workflow + RPA action + AI decision
#[derive(Clone)]
struct PipelineStep {
    id:          String,
    name:        String,
    kind:        String,  // "rpa", "ai_decision", "data_extract", "api_call", "human_task"
    // RPA config
    action:      Option<MacroAction>,
    // AI decision config
    prompt:      String,   // AI prompt for this step
    out_var:     String,   // variable to store AI response
    // Data extraction
    extract_pattern: String,  // regex or XPath-like selector
    extract_source:  String,  // "screen", "clipboard", "notification", "url"
    // Human task (pause and wait for signal)
    timeout_ms:  u128,
    // Retry
    retry_count: u32,
    retry_delay_ms: u64,
    // Condition to skip this step
    skip_if:     Option<MacroCondition>,
}

/// A hyper-automation pipeline
#[derive(Clone)]
struct HyperPipeline {
    id:          String,
    name:        String,
    steps:       Vec<PipelineStep>,
    enabled:     bool,
    run_count:   u64,
    last_run_ms: u128,
}

/// Retry result
enum RetryResult {
    Success(String),
    Failed(String, u32),  // error + attempts
}

/// Smart retry engine with exponential backoff (E-Robot pattern)
fn retry_action(
    s: &mut KiraState,
    macro_id: &str,
    action: &MacroAction,
    max_retries: u32,
    base_delay_ms: u64,
) -> RetryResult {
    for attempt in 0..=max_retries {
        // Enqueue the action
        enqueue_action(s, macro_id, action);
        // In Rust we can't actually wait for the Java result synchronously,
        // so we track retry state in variables
        let retry_key = format!("_retry_{}_{}", macro_id, action.kind.to_str());
        s.variables.insert(retry_key.clone(), AutoVariable {
            name: retry_key.clone(),
            value: format!("attempt:{}", attempt),
            var_type: "string".to_string(),
            persistent: false,
            created_ms: now_ms(),
            updated_ms: now_ms(),
        });
        if attempt < max_retries {
            // Exponential backoff: delay = base * 2^attempt (capped at 30s)
            let delay = (base_delay_ms * (1 << attempt.min(4))).min(30_000);
            s.pending_actions.push_back(PendingMacroAction {
                macro_id:  macro_id.to_string(),
                action_id: gen_id(),
                kind:      "wait".to_string(),
                params:    { let mut m = HashMap::new(); m.insert("ms".to_string(), delay.to_string()); m },
                ts: now_ms(),
            });
        }
    }
    RetryResult::Success("enqueued".to_string())
}

/// Execute a visual flowchart (Automate-style)
fn execute_flow(s: &mut KiraState, flow: &AutoFlow, start_id: Option<&str>) -> u32 {
    let mut steps = 0u32;
    let mut current_id = start_id.unwrap_or(&flow.start_block).to_string();
    let mut visited: HashMap<String, u32> = HashMap::new();
    let max_steps = 500u32;

    while steps < max_steps {
        let block = match flow.blocks.get(&current_id) {
            Some(b) => b.clone(),
            None => break,
        };

        // Loop guard \u{2014} prevent infinite loops
        let visit_count = visited.entry(current_id.clone()).or_insert(0);
        *visit_count += 1;
        if *visit_count > 100 { break; } // stuck in a loop

        steps += 1;

        match block.kind {
            FlowBlockKind::Stop => break,
            FlowBlockKind::Start => {
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Action => {
                if let Some(ref action) = block.action {
                    if block.retry_count > 0 {
                        retry_action(s, &flow.id, action, block.retry_count, block.retry_delay_ms);
                    } else {
                        enqueue_action(s, &flow.id, action);
                    }
                }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Decision => {
                let cond_result = block.condition.as_ref()
                    .map(|c| eval_condition(s, c))
                    .unwrap_or(false);
                current_id = if cond_result {
                    block.next.first().cloned().unwrap_or_default()
                } else {
                    block.next.get(1).cloned().unwrap_or_default()
                };
            }
            FlowBlockKind::Loop => {
                let count = block.loop_count.min(100);
                let body_id = block.next.first().cloned().unwrap_or_default();
                let after_id = block.next.get(1).cloned().unwrap_or_default();
                for i in 0..count {
                    // Set loop variable
                    if !block.loop_var.is_empty() {
                        let ts = now_ms();
                        s.variables.insert(block.loop_var.clone(), AutoVariable {
                            name: block.loop_var.clone(), value: i.to_string(),
                            var_type: "number".to_string(), persistent: false,
                            created_ms: ts, updated_ms: ts,
                        });
                    }
                    if !body_id.is_empty() {
                        let sub_flow = AutoFlow {
                            id: flow.id.clone(), name: flow.name.clone(),
                            description: String::new(), enabled: true,
                            start_block: body_id.clone(),
                            blocks: flow.blocks.clone(),
                            created_ms: 0, run_count: 0, last_run_ms: 0, tags: vec![],
                        };
                        steps += execute_flow(s, &sub_flow, Some(&body_id));
                    }
                }
                current_id = after_id;
            }
            FlowBlockKind::SubFlow => {
                if !block.sub_flow_id.is_empty() {
                    // Chain to another named flow
                    chain_macro(s, &block.sub_flow_id);
                }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Wait => {
                let ms = block.action.as_ref()
                    .and_then(|a| a.params.get("ms"))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1000);
                s.pending_actions.push_back(PendingMacroAction {
                    macro_id: flow.id.clone(), action_id: gen_id(),
                    kind: "wait".to_string(),
                    params: { let mut m = HashMap::new(); m.insert("ms".to_string(), ms.to_string()); m },
                    ts: now_ms(),
                });
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Fork => {
                // Parallel: enqueue all branches
                for next_id in &block.next {
                    let branch_flow = AutoFlow {
                        id: flow.id.clone(), name: flow.name.clone(),
                        description: String::new(), enabled: true,
                        start_block: next_id.clone(),
                        blocks: flow.blocks.clone(),
                        created_ms: 0, run_count: 0, last_run_ms: 0, tags: vec![],
                    };
                    steps += execute_flow(s, &branch_flow, Some(next_id));
                }
                break; // Fork doesn't have a single next
            }
            FlowBlockKind::Log => {
                let msg = block.label.clone();
                let expanded = expand_vars(s, &msg);
                s.daily_log.push_back(format!("[flow:{}] {}", flow.id, expanded));
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Catch => {
                // Error catch \u{2014} just continue to next
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Join => {
                current_id = block.next.first().cloned().unwrap_or_default();
            }
        }

        if current_id.is_empty() { break; }
    }
    steps
}

/// Execute a keyword (Robot Framework pattern)
/// Resolves args from variables then runs steps
fn execute_keyword(s: &mut KiraState, kw: &Keyword, args: &HashMap<String, String>) -> String {
    // Bind args to local variables
    for (name, val) in args {
        let ts = now_ms();
        s.variables.insert(name.clone(), AutoVariable {
            name: name.clone(), value: expand_vars(s, val),
            var_type: "string".to_string(), persistent: false,
            created_ms: ts, updated_ms: ts,
        });
    }
    // Run steps
    let steps = kw.steps.clone();
    let id = format!("kw_{}", kw.name.replace(' ', "_"));
    let (step_count, _) = execute_macro_actions(s, &id, &steps);
    // Return result variable
    if !kw.returns.is_empty() {
        s.variables.get(&kw.returns).map(|v| v.value.clone()).unwrap_or_default()
    } else {
        format!("ok:{}", step_count)
    }
}

/// Execute a hyper-automation pipeline (UiPath/Comidor pattern)
fn execute_pipeline(s: &mut KiraState, pipeline: &HyperPipeline) -> (u32, Vec<String>) {
    let mut steps = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for step in &pipeline.steps {
        // Check skip condition
        if let Some(ref cond) = step.skip_if {
            if eval_condition(s, cond) { continue; }
        }

        steps += 1;

        match step.kind.as_str() {
            "rpa" => {
                if let Some(ref action) = step.action {
                    if step.retry_count > 0 {
                        retry_action(s, &pipeline.id, action, step.retry_count, step.retry_delay_ms);
                    } else {
                        enqueue_action(s, &pipeline.id, action);
                    }
                }
            }
            "ai_decision" => {
                // Enqueue kira_ask action with the prompt
                let action = MacroAction {
                    kind: MacroActionKind::KiraAsk,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("prompt".to_string(), expand_vars(s, &step.prompt));
                        m.insert("out_var".to_string(), step.out_var.clone());
                        m
                    },
                    sub_actions: vec![],
                    enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            "data_extract" => {
                // Enqueue extraction action
                let mut params = HashMap::new();
                params.insert("source".to_string(), step.extract_source.clone());
                params.insert("pattern".to_string(), step.extract_pattern.clone());
                params.insert("out_var".to_string(), step.out_var.clone());
                let action = MacroAction {
                    kind: MacroActionKind::GetClipboard,
                    params, sub_actions: vec![], enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            "api_call" => {
                if let Some(ref action) = step.action {
                    enqueue_action(s, &pipeline.id, action);
                }
            }
            "human_task" => {
                // Pause pipeline and send notification to user
                let action = MacroAction {
                    kind: MacroActionKind::SendNotification,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("title".to_string(), format!("Action required: {}", step.name));
                        m.insert("text".to_string(), expand_vars(s, &step.prompt));
                        m
                    },
                    sub_actions: vec![], enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            _ => {
                errors.push(format!("unknown step kind: {}", step.kind));
            }
        }
    }
    (steps, errors)
}

/// Parse a flow from JSON
fn parse_flow_from_json(body: &str) -> Option<AutoFlow> {
    let id   = extract_json_str(body, "id").unwrap_or_else(gen_id);
    let name = extract_json_str(body, "name").unwrap_or_else(|| "Unnamed Flow".to_string());
    let desc = extract_json_str(body, "description").unwrap_or_default();
    let start= extract_json_str(body, "start_block").unwrap_or_default();
    if start.is_empty() { return None; }
    // blocks: [{id, kind, label, next:["id1","id2"], action:{...}, condition:{...}}]
    let mut blocks = HashMap::new();
    let blocks_key = r#""blocks":["#;
    let bstart = match body.find(blocks_key) {
        Some(i) => i + blocks_key.len(), None => return None
    };
    let slice = &body[bstart..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let bid  = extract_json_str(obj, "id").unwrap_or_else(gen_id);
                    let kind_str = extract_json_str(obj, "kind").unwrap_or_else(|| "action".to_string());
                    let label = extract_json_str(obj, "label").unwrap_or_default();
                    let loop_count = extract_json_num(obj, "loop_count").unwrap_or(1.0) as u32;
                    let loop_var   = extract_json_str(obj, "loop_var").unwrap_or_default();
                    let sub_flow_id= extract_json_str(obj, "sub_flow_id").unwrap_or_default();
                    let retry_count  = extract_json_num(obj, "retry_count").unwrap_or(0.0) as u32;
                    let retry_delay  = extract_json_num(obj, "retry_delay_ms").unwrap_or(1000.0) as u64;
                    // next array: ["id1","id2"]
                    let mut next_ids = Vec::new();
                    if let Some(ni) = obj.find(r#""next":["#) {
                        let ns = &obj[ni + 8..];
                        let end = ns.find(']').unwrap_or(ns.len());
                        for part in ns[..end].split(',') {
                            let id_part = part.trim().trim_matches('"').to_string();
                            if !id_part.is_empty() { next_ids.push(id_part); }
                        }
                    }
                    // Parse condition
                    let condition = if let Some(ci) = obj.find(r#""condition":{"#) {
                        let cs = &obj[ci + 13..];
                        let end = cs.find('}').unwrap_or(cs.len());
                        let co = &cs[..end];
                        Some(MacroCondition {
                            lhs: extract_json_str(co, "lhs").unwrap_or_default(),
                            operator: extract_json_str(co, "op").unwrap_or_else(|| "eq".to_string()),
                            rhs: extract_json_str(co, "rhs").unwrap_or_default(),
                        })
                    } else { None };
                    // Parse action
                    let action = if let Some(ai) = obj.find(r#""action":{"#) {
                        let ast = &obj[ai + 10..];
                        let end_act = find_matching_brace(ast).unwrap_or(ast.len());
                        let ao = &ast[..end_act];
                        let kind_s = extract_json_str(ao, "kind").unwrap_or_default();
                        let mut params = HashMap::new();
                        if let Some(pi) = ao.find(r#""params":{"#) {
                            let ps = &ao[pi + 10..];
                            let pe = ps.find('}').unwrap_or(ps.len());
                            parse_flat_kv(&ps[..pe], &mut params);
                        }
                        Some(MacroAction { kind: MacroActionKind::from_str(&kind_s), params, sub_actions: vec![], enabled: true })
                    } else { None };

                    let kind = match kind_str.as_str() {
                        "start"    => FlowBlockKind::Start,
                        "stop"     => FlowBlockKind::Stop,
                        "decision" => FlowBlockKind::Decision,
                        "loop"     => FlowBlockKind::Loop,
                        "wait"     => FlowBlockKind::Wait,
                        "fork"     => FlowBlockKind::Fork,
                        "join"     => FlowBlockKind::Join,
                        "sub_flow" => FlowBlockKind::SubFlow,
                        "catch"    => FlowBlockKind::Catch,
                        "log"      => FlowBlockKind::Log,
                        _          => FlowBlockKind::Action,
                    };
                    blocks.insert(bid.clone(), FlowBlock {
                        id: bid, kind, label, next: next_ids,
                        action, condition, loop_count, loop_var, sub_flow_id,
                        retry_count, retry_delay_ms: retry_delay,
                    });
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    Some(AutoFlow {
        id, name, description: desc, enabled: true, start_block: start,
        blocks, created_ms: now_ms(), run_count: 0, last_run_ms: 0, tags: vec![],
    })
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch { '{' => depth += 1, '}' => { depth -= 1; if depth < 0 { return Some(i); } } _ => {} }
    }
    None
}

/// Parse a keyword from JSON
fn parse_keyword_from_json(body: &str) -> Option<Keyword> {
    let name = extract_json_str(body, "name")?;
    let desc = extract_json_str(body, "description").unwrap_or_default();
    let returns = extract_json_str(body, "returns").unwrap_or_default();
    let args_str = extract_json_str(body, "args").unwrap_or_default();
    let args: Vec<String> = args_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let steps = parse_actions_from_json(body, "steps");
    Some(Keyword { name, description: desc, steps, args, returns })
}

/// Parse a pipeline from JSON
fn parse_pipeline_from_json(body: &str) -> Option<HyperPipeline> {
    let id   = extract_json_str(body, "id").unwrap_or_else(gen_id);
    let name = extract_json_str(body, "name").unwrap_or_else(|| "Pipeline".to_string());
    // steps: [{id, name, kind, prompt, out_var, retry_count, ...}]
    let mut steps = Vec::new();
    let key = r#""steps":["#;
    let start = match body.find(key) { Some(i) => i + key.len(), None => return None };
    let slice = &body[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let sid = extract_json_str(obj, "id").unwrap_or_else(gen_id);
                    let sname = extract_json_str(obj, "name").unwrap_or_default();
                    let kind  = extract_json_str(obj, "kind").unwrap_or_else(|| "rpa".to_string());
                    let prompt = extract_json_str(obj, "prompt").unwrap_or_default();
                    let out_var = extract_json_str(obj, "out_var").unwrap_or_default();
                    let extract_pattern = extract_json_str(obj, "extract_pattern").unwrap_or_default();
                    let extract_source  = extract_json_str(obj, "extract_source").unwrap_or_else(|| "screen".to_string());
                    let retry_count   = extract_json_num(obj, "retry_count").unwrap_or(0.0) as u32;
                    let retry_delay   = extract_json_num(obj, "retry_delay_ms").unwrap_or(1000.0) as u64;
                    let timeout_ms    = extract_json_num(obj, "timeout_ms").unwrap_or(30000.0) as u128;
                    let skip_if = if let Some(ci) = obj.find(r#""skip_if":{"#) {
                        let cs = &obj[ci + 11..]; let end = cs.find('}').unwrap_or(cs.len());
                        let co = &cs[..end];
                        Some(MacroCondition {
                            lhs: extract_json_str(co, "lhs").unwrap_or_default(),
                            operator: extract_json_str(co, "op").unwrap_or_else(|| "eq".to_string()),
                            rhs: extract_json_str(co, "rhs").unwrap_or_default(),
                        })
                    } else { None };
                    let action = if let Some(ai) = obj.find(r#""action":{"#) {
                        let ast = &obj[ai + 10..];
                        let end_act = find_matching_brace(ast).unwrap_or(ast.len());
                        let ao = &ast[..end_act];
                        let ks = extract_json_str(ao, "kind").unwrap_or_default();
                        let mut params = HashMap::new();
                        if let Some(pi) = ao.find(r#""params":{"#) {
                            let ps = &ao[pi+10..]; let pe = ps.find('}').unwrap_or(ps.len());
                            parse_flat_kv(&ps[..pe], &mut params);
                        }
                        Some(MacroAction { kind: MacroActionKind::from_str(&ks), params, sub_actions: vec![], enabled: true })
                    } else { None };
                    steps.push(PipelineStep { id: sid, name: sname, kind, action, prompt, out_var, extract_pattern, extract_source, timeout_ms, retry_count, retry_delay_ms: retry_delay, skip_if });
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    Some(HyperPipeline { id, name, steps, enabled: true, run_count: 0, last_run_ms: 0 })
}

/// HTTP routes for Roboru engine
fn route_roboru(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // Flows (visual flowchart)
        ("GET",  "/flows")          => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_flows.iter().map(|(id, f)|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"blocks":{},"run_count":{},"last_run_ms":{}}}"#,
                    esc(id), esc(&f.name), f.enabled, f.blocks.len(), f.run_count, f.last_run_ms)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/flows/add")      => {
            if let Some(flow) = parse_flow_from_json(body) {
                let id = flow.id.clone();
                STATE.lock().unwrap().roboru_flows.insert(id.clone(), flow);
                Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
            } else {
                Some(r#"{"error":"invalid flow json"}"#.to_string())
            }
        }
        ("POST", "/flows/run")      => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let flow = s.roboru_flows.get(&id).cloned();
            if let Some(flow) = flow {
                let steps = execute_flow(&mut s, &flow, None);
                if let Some(f) = s.roboru_flows.get_mut(&id) {
                    f.run_count += 1; f.last_run_ms = now_ms();
                }
                Some(format!(r#"{{"ok":true,"steps":{}}}"#, steps))
            } else {
                Some(format!(r#"{{"error":"flow not found: {}"}}"#, esc(&id)))
            }
        }
        ("POST", "/flows/remove")   => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            STATE.lock().unwrap().roboru_flows.remove(&id);
            Some(r#"{"ok":true}"#.to_string())
        }
        // Keywords (Robot Framework pattern)
        ("GET",  "/keywords")       => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_keywords.iter().map(|(name, kw)|
                format!(r#"{{"name":"{}","description":"{}","args":{},"steps":{}}}"#,
                    esc(name), esc(&kw.description),
                    format!("[{}]", kw.args.iter().map(|a| format!(r#""{}""#, esc(a))).collect::<Vec<_>>().join(",")),
                    kw.steps.len())
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/keywords/add")   => {
            if let Some(kw) = parse_keyword_from_json(body) {
                let name = kw.name.clone();
                STATE.lock().unwrap().roboru_keywords.insert(name.clone(), kw);
                Some(format!(r#"{{"ok":true,"name":"{}"}}"#, esc(&name)))
            } else { Some(r#"{"error":"invalid keyword json"}"#.to_string()) }
        }
        ("POST", "/keywords/run")   => {
            let name = extract_json_str(body, "name").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let kw = s.roboru_keywords.get(&name).cloned();
            if let Some(kw) = kw {
                let args: HashMap<String,String> = kw.args.iter().enumerate().map(|(i, arg_name)| {
                    let val = extract_json_str(body, &format!("arg{}", i)).unwrap_or_default();
                    (arg_name.clone(), val)
                }).collect();
                let result = execute_keyword(&mut s, &kw, &args);
                Some(format!(r#"{{"ok":true,"result":"{}"}}"#, esc(&result)))
            } else { Some(format!(r#"{{"error":"keyword not found: {}"}}"#, esc(&name))) }
        }
        // Pipelines (Hyper-automation)
        ("GET",  "/pipelines")      => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_pipelines.iter().map(|(id, p)|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"steps":{},"run_count":{}}}"#,
                    esc(id), esc(&p.name), p.enabled, p.steps.len(), p.run_count)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/pipelines/add")  => {
            if let Some(pipeline) = parse_pipeline_from_json(body) {
                let id = pipeline.id.clone();
                STATE.lock().unwrap().roboru_pipelines.insert(id.clone(), pipeline);
                Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
            } else { Some(r#"{"error":"invalid pipeline json"}"#.to_string()) }
        }
        ("POST", "/pipelines/run")  => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let pipeline = s.roboru_pipelines.get(&id).cloned();
            if let Some(pipeline) = pipeline {
                let (steps, errors) = execute_pipeline(&mut s, &pipeline);
                if let Some(p) = s.roboru_pipelines.get_mut(&id) {
                    p.run_count += 1; p.last_run_ms = now_ms();
                }
                Some(format!(r#"{{"ok":true,"steps":{},"errors":{}}}"#,
                    steps,
                    format!("[{}]", errors.iter().map(|e| format!(r#""{}""#, esc(e))).collect::<Vec<_>>().join(","))))
            } else { Some(format!(r#"{{"error":"pipeline not found: {}"}}"#, esc(&id))) }
        }
        _ => None,
    }
}

// \u{2500}\u{2500}\u{2500} OpenClaw v2: Advanced automation features \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Macro schedule: run macro at specific time daily (HH:MM format)
/// Stored as a cron job internally
fn schedule_macro_daily(s: &mut KiraState, macro_id: &str, time_hhmm: &str) {
    // Parse HH:MM \u{2192} store as cron job with interval = 24h
    // The trigger watcher checks against current time string
    let job_id = format!("daily_{}_{}", macro_id, time_hhmm.replace(':', ""));
    s.cron_jobs.retain(|j| j.id != job_id);
    s.cron_jobs.push(CronJob {
        id:          job_id,
        expression:  time_hhmm.to_string(),
        action:      format!("chain_macro:{}", macro_id),
        last_run:    0,
        interval_ms: 86_400_000, // 24h
        enabled:     true,
    });
}

/// Macro group: run multiple macros in sequence or parallel
fn run_macro_group(s: &mut KiraState, macro_ids: &[&str], parallel: bool) {
    if !check_rate_limit(s) { return; }
    if parallel {
        // Enqueue all at once \u{2014} Java executes them concurrently
        for id in macro_ids {
            let actions: Vec<MacroAction> = s.macros.iter()
                .find(|m| m.id == *id && m.enabled)
                .map(|m| m.actions.clone())
                .unwrap_or_default();
            if !actions.is_empty() {
                let (_, _) = execute_macro_actions(s, id, &actions);
            }
        }
    } else {
        // Sequential: chain them
        for id in macro_ids {
            chain_macro(s, id);
        }
    }
}

/// Conditional macro: only run if ALL conditions pass
fn try_run_macro_conditional(s: &mut KiraState, macro_id: &str) -> bool {
    let conditions: Vec<MacroCondition> = s.macros.iter()
        .find(|m| m.id == macro_id)
        .map(|m| m.conditions.clone())
        .unwrap_or_default();
    if !conditions.iter().all(|c| eval_condition(s, c)) { return false; }
    chain_macro(s, macro_id);
    true
}

/// Smart trigger debounce: ignore repeat fires within N ms
fn is_debounced(s: &KiraState, macro_id: &str, debounce_ms: u128) -> bool {
    let now = now_ms();
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        return now - m.last_run_ms < debounce_ms;
    }
    false
}

/// Variable interpolation in action params \u{2014} supports math expressions
fn resolve_param(s: &KiraState, param: &str) -> String {
    let expanded = expand_vars(s, param);
    // If it looks like an expression (has operators), try to evaluate
    if expanded.contains('+') || expanded.contains('-') ||
       expanded.contains('*') || expanded.contains('/') {
        if let Some(result) = eval_math(expanded.trim()) {
            return result;
        }
    }
    expanded
}

/// Get macro by name (case-insensitive) \u{2014} useful for natural language commands
fn find_macro_by_name(s: &KiraState, name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    s.macros.iter()
        .find(|m| m.name.to_lowercase().contains(&lower) || m.id == name)
        .map(|m| m.id.clone())
}

/// Automation analytics: return summary of recent macro activity
fn get_automation_analytics(s: &KiraState) -> String {
    let now = now_ms();
    let last_24h = s.macro_run_log.iter()
        .filter(|r| now - r.ts < 86_400_000).count();
    let last_1h = s.macro_run_log.iter()
        .filter(|r| now - r.ts < 3_600_000).count();
    let success_count = s.macro_run_log.iter()
        .filter(|r| r.success).count();
    let fail_count = s.macro_run_log.iter()
        .filter(|r| !r.success).count();
    let total_steps: u32 = s.macro_run_log.iter().map(|r| r.steps_run).sum();
    let enabled_macros = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
    let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();

    // Most active macro
    let mut counts: HashMap<String, u32> = HashMap::new();
    for r in &s.macro_run_log {
        *counts.entry(r.macro_name.clone()).or_insert(0) += 1;
    }
    let most_active = counts.iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, v)| format!("{} ({}x)", k, v))
        .unwrap_or_else(|| "none".to_string());

    format!(
        r#"{{"runs_24h":{},"runs_1h":{},"success":{},"failed":{},"total_steps":{},"enabled_macros":{},"templates":{},"variables":{},"active_profile":"{}","most_active":"{}","pending_actions":{}}}"#,
        last_24h, last_1h, success_count, fail_count, total_steps,
        enabled_macros, templates, s.variables.len(),
        esc(&s.active_profile), esc(&most_active),
        s.pending_actions.len()
    )
}

/// Automation report: full text summary for AI to read
fn get_automation_report(s: &KiraState) -> String {
    let now = now_ms();
    let mut lines = Vec::new();
    lines.push(format!("=== Kira Automation Report ==="));
    lines.push(format!("Active profile: {}", s.active_profile));
    lines.push(format!("Enabled macros: {}", s.macros.iter().filter(|m| m.enabled).count()));
    lines.push(format!("Total variables: {}", s.variables.len()));
    lines.push(format!("Pending actions: {}", s.pending_actions.len()));
    lines.push(String::new());
    lines.push("Recent runs:".to_string());
    for r in s.macro_run_log.iter().rev().take(5) {
        let ago = (now - r.ts) / 1000;
        lines.push(format!("  \u{2022} {} \u{2014} {} steps \u{2014} {}s ago", r.macro_name, r.steps_run, ago));
    }
    lines.push(String::new());
    lines.push("Variables:".to_string());
    for (name, var) in s.variables.iter().take(10) {
        lines.push(format!("  %{}% = {}", name.to_uppercase(), var.value));
    }
    lines.join("\n")
}

/// Export all macros as a single JSON string (for backup / sharing)
fn export_macros_json(s: &KiraState) -> String {
    let items: Vec<String> = s.macros.iter().map(macro_to_json).collect();
    format!(
        r#"{{"version":"9.0","exported_ms":{},"count":{},"macros":[{}]}}"#,
        now_ms(), items.len(), items.join(",")
    )
}

/// Import macros from exported JSON (merge, don't wipe existing)
fn import_macros_json(s: &mut KiraState, json: &str) {
    // Find the "macros":[...] array and parse each entry
    let key = "\"macros\":[";
    let start = match json.find(key) { Some(i) => i + key.len(), None => return };
    let slice = &json[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let m = parse_macro_from_json(obj);
                    s.macros.retain(|x| x.id != m.id); // replace if exists
                    s.macros.push(m);
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
}

/// Watchdog: find macros that have been pending for >30s and log them
fn watchdog_check(s: &mut KiraState) {
    let now = now_ms();
    let stale: Vec<String> = s.pending_actions.iter()
        .filter(|a| now - a.ts > 30_000)
        .map(|a| format!("{}:{}", a.macro_id, a.kind))
        .collect();
    if !stale.is_empty() {
        s.daily_log.push_back(format!("[watchdog] stale actions: {}", stale.join(", ")));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
        // Remove stale actions older than 2 minutes
        s.pending_actions.retain(|a| now - a.ts < 120_000);
    }
}

/// HTTP route additions for OpenClaw features
fn route_openclaw(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        ("GET",  "/macros/export")  => Some(export_macros_json(&STATE.lock().unwrap())),
        ("POST", "/macros/import")  => { import_macros_json(&mut STATE.lock().unwrap(), body); Some(r#"{"ok":true}"#.to_string()) }
        ("GET",  "/macros/templates") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macros.iter()
                .filter(|m| m.tags.contains(&"template".to_string()))
                .map(macro_to_json).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/macros/chain")   => {
            let id = extract_json_str(body, "target").unwrap_or_default();
            if !id.is_empty() { chain_macro(&mut STATE.lock().unwrap(), &id); }
            Some(format!(r#"{{"ok":true,"chained":"{}"}}"#, esc(&id)))
        }
        ("POST", "/macros/pipeline") => {
            let id = extract_json_str(body, "macro_id").unwrap_or_else(gen_id);
            run_pipeline(&mut STATE.lock().unwrap(), &id, body);
            Some(format!(r#"{{"ok":true,"pipeline":"{}"}}"#, esc(&id)))
        }
        ("GET",  "/expr")           => {
            // Evaluate expression: GET /expr?e=5+3 \u{2192} {"result":"8"}
            let expr = path.find("e=").map(|i| &path[i+2..]).unwrap_or("").replace('+', " ");
            let result = eval_expr(&STATE.lock().unwrap(), &expr);
            Some(format!(r#"{{"result":"{}"}}"#, esc(&result)))
        }
        ("GET",  "/variables/expand") => {
            // Expand %VAR% tokens: GET /variables/expand?text=hello+%BATTERY%
            let text = path.find("text=").map(|i| &path[i+5..]).unwrap_or("").replace('+', " ");
            let result = expand_vars(&STATE.lock().unwrap(), &text);
            Some(format!(r#"{{"result":"{}"}}"#, esc(&result)))
        }
        ("GET",  "/automation/analytics") => Some(get_automation_analytics(&STATE.lock().unwrap())),
        ("GET",  "/automation/report")    => {
            let report = get_automation_report(&STATE.lock().unwrap());
            Some(format!(r#"{{"report":"{}"}}"#, esc(&report)))
        }
        ("POST", "/macros/schedule")      => {
            let id   = extract_json_str(body, "macro_id").unwrap_or_default();
            let time = extract_json_str(body, "time").unwrap_or_default();
            if !id.is_empty() && !time.is_empty() {
                schedule_macro_daily(&mut STATE.lock().unwrap(), &id, &time);
            }
            Some(format!(r#"{{"ok":true,"scheduled":"{}","time":"{}"}}"#, esc(&id), esc(&time)))
        }
        ("POST", "/macros/group")         => {
            let parallel = body.contains(r#""parallel":true"#);
            let ids_str = extract_json_str(body, "ids").unwrap_or_default();
            let ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();
            run_macro_group(&mut STATE.lock().unwrap(), &ids, parallel);
            Some(format!(r#"{{"ok":true,"count":{}}}"#, ids.len()))
        }
        ("GET",  "/macros/find")          => {
            let name = body.find("name=").map(|i| &body[i+5..]).unwrap_or("").split('&').next().unwrap_or("");
            let result = find_macro_by_name(&STATE.lock().unwrap(), name);
            Some(match result {
                Some(id) => format!(r#"{{"found":true,"id":"{}"}}"#, esc(&id)),
                None     => r#"{"found":false}"#.to_string(),
            })
        }
        ("POST", "/macros/conditional")   => {
            let id = extract_json_str(body, "macro_id").unwrap_or_default();
            let ran = if !id.is_empty() { try_run_macro_conditional(&mut STATE.lock().unwrap(), &id) } else { false };
            Some(format!(r#"{{"ok":true,"ran":{}}}"#, ran))
        }
        ("GET",  "/automation/status") => {
            let s = STATE.lock().unwrap();
            let enabled = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
            let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();
            Some(format!(
                r#"{{"enabled_macros":{},"templates":{},"total_macros":{},"variables":{},"active_profile":"{}","pending_actions":{},"run_log_entries":{},"rate_ok":{}}}"#,
                enabled, templates, s.macros.len(), s.variables.len(),
                esc(&s.active_profile), s.pending_actions.len(),
                s.macro_run_log.len(), check_rate_limit(&s)
            ))
        }
        _ => None,
    }
}

// Crypto
// SECURITY: Credential key derivation — 1024 rounds of byte mixing.
// This is obfuscation (in-memory protection), NOT cryptographic encryption.
// For real encryption-at-rest, use Android Keystore via JNI.
fn derive_key(name: &str) -> Vec<u8> {
    let mut key = [0u8; 32];
    for (i, b) in name.bytes().enumerate() {
        key[i % 32] = key[i % 32].wrapping_add(b).wrapping_add(i as u8);
    }
    // Stretch: 1024 mixing rounds
    for round in 0u32..1024 {
        let rb = (round & 0xFF) as u8;
        for i in 0..32usize {
            key[i] = key[i]
                .wrapping_add(key[(i + 1) % 32])
                .wrapping_add(rb)
                .rotate_left(((i % 7) + 1) as u32);
        }
    }
    key.to_vec()
}
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() { return data.to_vec(); }
    let klen = key.len();
    data.iter().enumerate().map(|(i, &b)| {
        let k1 = key[i % klen];
        let k2 = key[(i.wrapping_add(klen / 2 + 1)) % klen];
        b ^ k1 ^ k2.rotate_left(((i % 5) + 1) as u32)
    }).collect()
}

// Utilities
