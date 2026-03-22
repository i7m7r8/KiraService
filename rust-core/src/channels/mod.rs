// kira-core :: channels  (Sessions 7+8)
pub mod telegram;
pub mod whatsapp;
pub mod shared;

pub use shared::{InboundMessage, OutboundMessage, ChannelId, SendResult, DmPolicy};
pub use telegram::{
    TG_STATE, TelegramConfig, TgUpdate, TgOutbound, TgRuntime,
    register_tg_fns, start_polling_loop,
    send_message as tg_send, edit_message as tg_edit,
    send_typing as tg_typing, send_with_keyboard as tg_send_keyboard,
    answer_callback as tg_answer_callback,
    escape_md_v2, markdown_to_md_v2, parse_updates as tg_parse_updates,
};
pub use whatsapp::{
    WA_STATE, WhatsAppConfig, WaInbound, WaOutbound, WaRuntime,
    register_wa_fns,
    cloud_send_text, cloud_mark_read, bridge_queue_send,
    process_inbound as wa_process_inbound,
    parse_cloud_webhook,
};
