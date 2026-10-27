/**
 * ZapZap (Rust) local API page bridge.
 *
 * Exposes a tiny, defensive read-and-act surface over the WhatsApp Web DOM
 * for the local automation API. Every entry point returns a plain JSON-able
 * object and never throws: the automation layer must not take the embedded
 * page down when WhatsApp changes its markup.
 *
 * Injected as a resident main-world initialization script by the API module.
 *
 * Response transport (in order of preference):
 *   1. window.__zapdude.respond(serializedJson) - native bridge object when
 *      the host exposes one (WebView2 host-injected helper).
 *   2. location hash beacon (`#__zapdude=<urlencoded payload>`) polled by the
 *      host through WebviewWindow::url().
 */
(function () {
    'use strict';

    var BRIDGE_ID = '__zapZapApiV1';

    function emitError(operation, error) {
        return {
            ok: false,
            error: 'unexpected_error',
            detail: String((error && error.message) || error || ''),
            operation: operation,
        };
    }

    function findComposer() {
        const selectors = [
            'footer div[contenteditable="true"]',
            'div[role="textbox"][data-tab="10"]',
            'div[role="textbox"][data-tab="6"]',
            'div[role="textbox"][contenteditable="true"]',
            'div[contenteditable="true"][data-tab]',
        ];
        for (const selector of selectors) {
            const nodes = document.querySelectorAll(selector);
            for (const node of nodes) {
                try {
                    if (node.offsetParent !== null && !node.closest('div[role="dialog"]')) {
                        return node;
                    }
                } catch (_hiddenCheckError) {
                    return node;
                }
            }
        }
        return null;
    }

    function decodeFinalUrl() {
        try {
            return String(location.href || '');
        } catch (_error) {
            return '';
        }
    }

    function visibleChatListItems() {
        const hosts = [
            '#pane-side div[role="listitem"]',
            'div[role="listitem"]',
        ];
        let items = [];
        for (const selector of hosts) {
            items = Array.from(document.querySelectorAll(selector));
            if (items.length) break;
        }
        return items;
    }

    function scrapeChatList(limit) {
        try {
            const max = Math.max(1, Math.min(100, Number(limit) || 20));
            const items = visibleChatListItems();
            const chats = [];
            for (const item of items.slice(0, max)) {
                let title = '';
                let unread = false;
                let snippet = '';
                try {
                    const titleNode = item.querySelector('span[title]');
                    title = titleNode ? titleNode.getAttribute('title') : '';
                    const badge = item.querySelector('[aria-label*="read"], [aria-label*="lidos"], [aria-label*="no ley"]');
                    const badgeMatch = badge ? String(badge.getAttribute('aria-label') || '').match(/(\d+)/) : null;
                    unread = (badge && !!badgeMatch) || !!item.querySelector('span[aria-label*="missed"]');
                    if (badgeMatch) unread = true;
                    snippet = titleNode ? String(titleNode.parentElement ? titleNode.parentElement.innerText || '' : '') : String(item.innerText || '');
                    if (snippet.length > 120) snippet = snippet.slice(0, 120);
                } catch (_itemParseError) {
                    // Skip malformed items; the list remains best-effort.
                }
                chats.push({ title: title, unread: unread, snippet: snippet });
            }
            return { ok: true, count: chats.length, chats: chats, degraded: chats.length === 0 };
        } catch (error) {
            return emitError('chatList', error);
        }
    }

    function pageScript() {
        const api = {
            version: 1,

            /** Report the composer state without touching the page. */
            composerInfo: function () {
                try {
                    const composer = findComposer();
                    if (!composer) {
                        return {
                            ok: true,
                            ready: false,
                            reason: 'not_ready',
                            chat_open: document.querySelector('[aria-label="Chat list"], #pane-side, div[role="listitem"]') != null,
                            title: String(document.title || ''),
                        };
                    }
                    let textLength = 0;
                    try {
                        textLength = String(composer.innerText || '').length;
                    } catch (_innerTextError) {
                        textLength = 0;
                    }
                    return {
                        ok: true,
                        ready: true,
                        empty: textLength === 0,
                        text_length: textLength,
                        title: String(document.title || ''),
                    };
                } catch (error) {
                    return emitError('composerInfo', error);
                }
            },

            /** Outline page level state for status/debug endpoints. */
            pageInfo: function () {
                try {
                    return {
                        ok: true,
                        url: decodeFinalUrl(),
                        title: String(document.title || ''),
                        chat_list: !!document.querySelector('[aria-label="Chat list"], #pane-side, div[role="listitem"]'),
                        composer: !!findComposer(),
                        qrcode: document.querySelectorAll('canvas').length > 0,
                    };
                } catch (error) {
                    return emitError('pageInfo', error);
                }
            },

            /**
             * Insert text into the open composer and dispatch Enter.
             * Mirrors the resilient paste strategy used by ZapZap's own
             * paste-as-plain-text feature (Lexical-aware).
             */
            sendCurrent: function (text) {
                try {
                    const composer = findComposer();
                    if (!composer) {
                        return { ok: false, error: 'composer_not_found' };
                    }
                    try {
                        composer.focus({ preventScroll: true });
                    } catch (_focusError) {
                        composer.focus();
                    }
                    function normalizeForCompare(value) {
                        return String(value || '')
                            .replace(/[\u200b-\u200d\u200e\u200f\ufeff]/g, '')
                            .replace(/\s+/g, ' ')
                            .trim();
                    }
                    let inserted = false;
                    let skippedInsert = false;
                    const already = normalizeForCompare(composer.innerText);
                    const wanted = normalizeForCompare(text);
                    if (wanted && (already === wanted || already.endsWith(wanted))) {
                        // WhatsApp prefilled the composer from the URL; the
                        // message is already typed, only the send press remains.
                        skippedInsert = true;
                    }

                    // Controlled Lexical editors prefer a synthetic paste.
                    if (!skippedInsert && composer.hasAttribute('data-lexical-editor')) {
                        try {
                            const transfer = new DataTransfer();
                            transfer.setData('text/plain', text);
                            const pasteEvent = new ClipboardEvent('paste', {
                                clipboardData: transfer,
                                bubbles: true,
                                cancelable: true,
                            });
                            composer.dispatchEvent(pasteEvent);
                            inserted = pasteEvent.defaultPrevented;
                        } catch (_lexPasteError) {
                            inserted = false;
                        }
                    }

                    if (!inserted) {
                        try {
                            if (text.indexOf('\n') !== -1) {
                                const lines = text.split('\n');
                                let wrote = false;
                                for (const [index, line] of lines.entries()) {
                                    if (index > 0) {
                                        if (!document.execCommand('insertLineBreak', false, null)) break;
                                    }
                                    if (line) {
                                        if (!document.execCommand('insertText', false, line)) break;
                                    }
                                    wrote = true;
                                }
                                inserted = wrote;
                            } else {
                                inserted = document.execCommand('insertText', false, text);
                            }
                        } catch (_execCommandError) {
                            inserted = false;
                        }
                    }

                    let pressed = true;
                    try {
                        const base = {
                            key: 'Enter',
                            code: 'Enter',
                            keyCode: 13,
                            which: 13,
                            bubbles: true,
                            cancelable: true,
                            composed: true,
                        };
                        composer.dispatchEvent(new KeyboardEvent('keydown', base));
                        composer.dispatchEvent(new KeyboardEvent('keypress', base));
                        composer.dispatchEvent(new KeyboardEvent('keyup', base));
                    } catch (_enterError) {
                        pressed = false;
                    }

                    if (!pressed) {
                        // Last resort: click the visible send button.
                        try {
                            const buttons = document.querySelectorAll(
                                'button[data-tab="send"], span[data-icon="send"], button[aria-label]'
                            );
                            for (const button of buttons) {
                                if (String(button.getAttribute('aria-label') || '').includes('end')
                                    || !!button.querySelector('span[data-icon="send"]')) {
                                    button.click();
                                    pressed = true;
                                    break;
                                }
                            }
                        } catch (_clickFallbackError) {
                            pressed = false;
                        }
                    }

                    return {
                        ok: true,
                        inserted: inserted,
                        skipped_insert: skippedInsert,
                        pressed: pressed,
                        text_length: text.length,
                    };
                } catch (error) {
                    return emitError('sendCurrent', error);
                }
            },

            /** Best-effort scrape of the visible chat list. */
            chatList: function (limit) {
                return scrapeChatList(limit);
            },

            /**
             * Dispatch an incoming message event to the host.
             * Runs from the Notification hook and chat list observers.
             */
            pushEvent: function (eventJson) {
                var event = null;
                try {
                    event = JSON.parse(eventJson);
                } catch (_error) {
                    event = { kind: 'unexpected' };
                }
                emitResponse('evt_' + String(Date.now()) + '_' + Math.floor(Math.random() * 1e6), event);
                return { ok: true };
            },
        };

        function hashBeacon(payload) {
            try {
                location.replace('#__zapdude=' + encodeURIComponent(payload));
                return true;
            } catch (_beaconError) {
                return false;
            }
        }

        function emitResponse(id, payload) {
            var envelope = JSON.stringify({ id: String(id), payload: payload });
            try {
                // WebView2 host helpers may be exposed via AddHostObject once
                // a later version enables them; prefer direct call first.
                if (window.__zapdude && typeof window.__zapdude.respond === 'function') {
                    window.__zapdude.respond(envelope);
                    return;
                }
            } catch (_respondError) {
                /* fall through to beacon */
            }
            hashBeacon(envelope);
        }

        function dispatchCommand(id, op, argsJson) {
            var args = {};
            try {
                args = JSON.parse(argsJson || '{}') || {};
            } catch (_parseError) {
                args = {};
            }
            var payload;
            try {
                if (op === 'composerInfo') {
                    payload = api.composerInfo();
                } else if (op === 'pageInfo') {
                    payload = api.pageInfo();
                } else if (op === 'sendCurrent') {
                    payload = api.sendCurrent(String(args.text || ''));
                } else if (op === 'chatList') {
                    payload = api.chatList(Number(args.limit) || 20);
                } else {
                    payload = { ok: false, error: 'unknown_operation', operation: String(op || '') };
                }
            } catch (error) {
                payload = emitError(String(op || 'unknown'), error);
            }
            emitResponse(id, payload);
            return { ok: true };
        }

        window.__zapZapApiV1 = api;
        api.dispatch = function (id, op, argsJson) {
            try {
                return dispatchCommand(id, op, argsJson);
            } catch (error) {
                emitResponse(id, emitError(String(op || 'unknown'), error));
                return { ok: false };
            }
        };

        // ------------------------------------------------------------------
        // Incoming event sources
        // ------------------------------------------------------------------
        var lastNotificationTitle = null;

        function pushIncomingMessage(kind, rawTitle, rawBody) {
            try {
                var title = String(rawTitle || '');
                var body = String(rawBody || '');
                if (kind === 'notification' && title && title === lastNotificationTitle && !body) {
                    return; // collapse identical repeated notifications
                }
                if (kind === 'notification') {
                    lastNotificationTitle = title;
                }
                api.pushEvent(JSON.stringify({
                    kind: kind,
                    title: title,
                    body: body,
                    at: Math.floor(Date.now() / 1000),
                }));
            } catch (_pushError) {
                // Never break the page for an event failure.
            }
        }

        try {
            var NativeNotification = window.Notification;
            function ZapZapNotification(title, options) {
                try {
                    pushIncomingMessage('notification', title, options && options.body);
                    if (NativeNotification) {
                        return new NativeNotification(title, options);
                    }
                } catch (_notifyError) {
                    return null;
                }
                return null;
            }
            if (NativeNotification) {
                ZapZapNotification.prototype = NativeNotification.prototype;
                ZapZapNotification.permission = NativeNotification.permission;
                ZapZapNotification.requestPermission = function (callback) {
                    return NativeNotification.requestPermission(callback);
                };
            }
            window.Notification = ZapZapNotification;
        } catch (_hookError) {
            // Notifications cannot be hooked without breaking the page.
        }

        try {
            var observer = new MutationObserver(function () {
                try {
                    var chats = scrapeChatList(40);
                    if (chats && chats.ok && chats.count > 0 && !chats.degraded) {
                        api.pushEvent(JSON.stringify({ kind: 'chat_list', chats: chats }));
                    }
                } catch (_observeError) {
                    /* ignore */
                }
            });
            observer.observe(document, { childList: true, subtree: true, attributes: false });
        } catch (_observerError) {
            // The observer is an enhancement, not a requirement.
        }
    }

    try {
        pageScript();
    } catch (_setupError) {
        // A bridge failure must never disturb the embedded page.
    }
})();
