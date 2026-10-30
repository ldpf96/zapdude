/**
 * Zapdude notification capture.
 *
 * Replaces the page-level Notification API with a stub that forwards every
 * notification to the host through the beacon transport and never asks
 * WebView2 to render its web-styled bubble (the look the user called out:
 * "salen como notificaciones web"). The host renders the UNIFORM native
 * toast (Windows WinRT via src/notifier.rs) - the single surface whether
 * the window is visible or hidden.
 *
 * The original constructor is NOT called, so WebView2's own toast never
 * shows. Permission reads report 'granted' and requestPermission resolves
 * 'granted' so WhatsApp keeps shipping notifications this way.
 *
 * Click support: the host calls window.__zapdudeFireNotificationClick(tag)
 * and the stub dispatches the stored onclick handlers, so "click
 * notification -> WhatsApp opens the chat" keeps working with our own
 * surface.
 *
 * Installed DEFERRED by the beacon poller with its own idempotency flag and
 * try/catch (hooking at document-start once broke WhatsApp's startup; the
 * deferred slot is the proven safe installation point).
 */
(function () {
    'use strict';

    if (window.__zapdudeNotifyV1) {
        return;
    }
    window.__zapdudeNotifyV1 = true;

    var INSTANCES = [];

    function shipBeacon(envelope) {
        try {
            location.replace('#__zapdude=' + encodeURIComponent(JSON.stringify(envelope)));
            return true;
        } catch (_beaconError) {
            return false;
        }
    }

    function emit(kind, title, options) {
        // Missed-call payloads carry the caller's bare HANDLE as the
        // title (WhatsApp's own choice). The ring relay teaches us the
        // handle -> display-name pairing just before the missed call
        // arrives, so the toast gets the human identity.
        var origTitle = String(title || '');
        var shownTitle = origTitle;
        var shownIcon = '';
        try {
            if (shownTitle.charAt(0) === '@') {
                var map = window.__zapdudeCallerMap;
                if (map) {
                    var mapped = map[shownTitle];
                    if (!mapped) {
                        // Stored keys may carry fused trailing text from the
                        // ring surface nodes ("@ldpfpeVoice") - a title of
                        // "@ldpfpe" prefix-matches such a key.
                        for (var key in map) {
                            if (Object.prototype.hasOwnProperty.call(map, key)
                                && key.indexOf(shownTitle) === 0) {
                                mapped = map[key];
                                break;
                            }
                        }
                    }
                    if (mapped && mapped.length >= 2 && mapped.length <= 40) {
                        shownTitle = mapped;
                    }
                }
                // The missed-call Notification ships WhatsApp's OWN icon: a
                // composite bitmap (photo re-cropped + WhatsApp badge
                // stamped on) that breaks the round-brand toast look (user
                // report, 2026-10-31). The ring surface that JUST preceded
                // this call stored its clean channel avatar under the same
                // handle key - reuse it so the missed toast wears the exact
                // face the incoming toast wore. No ring seen -> keep
                // WhatsApp's icon (degrade, never crash).
                var icons = window.__zapdudeCallerIcon;
                if (icons) {
                    var missIcon = icons[origTitle];
                    if (!missIcon) {
                        for (var iconKey in icons) {
                            if (Object.prototype.hasOwnProperty.call(icons, iconKey)
                                && iconKey.indexOf(origTitle) === 0) {
                                missIcon = icons[iconKey];
                                break;
                            }
                        }
                    }
                    if (missIcon && missIcon.length <= 300000
                        && (/^https:\/\//.test(missIcon) || /^data:/.test(missIcon))) {
                        shownIcon = missIcon;
                    }
                }
            }
        } catch (_mapError) {}
        var payload = {
            kind: kind,
            title: shownTitle,
            body: String((options && options.body) || ''),
            icon: shownIcon || String((options && options.icon) || ''),
            tag: String((options && options.tag) || ''),
            lang: String((options && options.lang) || ''),
            require_interaction: !!(options && options.requireInteraction),
            silent: !!(options && options.silent),
        };
        try {
            if (window.__zapdudeApiV1 && typeof window.__zapdudeApiV1.pushEvent === 'function') {
                window.__zapdudeApiV1.pushEvent(JSON.stringify(payload));
                return;
            }
        } catch (_bridgeError) {
            /* fall through to the raw beacon */
        }
        shipBeacon({
            id: 'evt_' + String(Date.now()) + '_' + Math.floor(Math.random() * 1e6),
            payload: payload,
        });
    }

    function dropInstance(target) {
        INSTANCES = INSTANCES.filter(function (entry) {
            return entry !== target;
        });
    }

    function ZapdudeNotification(title, options) {
        if (!(this instanceof ZapdudeNotification)) {
            // Called without `new`: behave as a factory rather than throw.
            return new ZapdudeNotification(title, options);
        }
        var title0 = String(title || '');
        var options0 = (options && typeof options === 'object') ? options : {};
        var self = this;
        Object.defineProperties(this, {
            title: { value: title0 },
            dir: { value: String(options0.dir || 'auto') },
            lang: { value: String(options0.lang || '') },
            body: { value: String(options0.body || '') },
            tag: { value: String(options0.tag || '') },
            icon: { value: String(options0.icon || '') },
            silent: { value: !!options0.silent },
            data: {
                value: options0.data === undefined ? null : options0.data,
            },
            requireInteraction: { value: !!options0.requireInteraction },
            onclick: { writable: true, value: null },
            onshow: { writable: true, value: null },
            onerror: { writable: true, value: null },
            onclose: { writable: true, value: null },
        });
        var listeners = [];
        this.addEventListener = function (type, handler) {
            try {
                listeners.push({ type: type, handler: handler });
            } catch (_listenError) {}
        };
        this.removeEventListener = function (type, handler) {
            try {
                listeners = listeners.filter(function (entry) {
                    return entry.type !== type || entry.handler !== handler;
                });
            } catch (_unlistenError) {}
        };
        this.dispatchEvent = function (event) {
            try {
                var type = String((event && event.type) || 'click');
                listeners.slice().forEach(function (entry) {
                    if (entry.type === type && typeof entry.handler === 'function') {
                        entry.handler.call(this, event);
                    }
                }, this);
                return !(
                    event && event.defaultPrevented
                );
            } catch (_dispatchError) {
                return true;
            }
        };
        INSTANCES.push(self);
        emit('notification', title0, options0);
    }

    ZapdudeNotification.prototype.close = function () {
        emit('notification_closed', this.title, { tag: this.tag });
        dropInstance(this);
        if (typeof this.onclose === 'function') {
            try {
                this.onclose.call(this, { type: 'close' });
            } catch (_closeError) {}
        }
    };

    Object.defineProperty(ZapdudeNotification, 'permission', {
        get: function () { return 'granted'; },
    });
    ZapdudeNotification.requestPermission = function (callback) {
        var promise = Promise.resolve('granted');
        if (typeof callback === 'function') {
            Promise.resolve(promise).then(function (state) {
                callback(state);
            });
        }
        return promise;
    };
    ZapdudeNotification.maxActions = 2;

    window.Notification = ZapdudeNotification;

    // Host -> page: fire the stored click handlers for one tag (empty tag
    // matches every notification with an empty tag). One call per toast
    // click; the host delivers the account focus itself first.
    window.__zapdudeFireNotificationClick = function (tag) {
        try {
            var wanted = String(tag === undefined || tag === null ? '' : tag);
            var fired = 0;
            INSTANCES.slice().forEach(function (instance) {
                if (String(instance.tag || '') === wanted) {
                    if (typeof instance.onclick === 'function') {
                        instance.onclick.call(instance, { type: 'click' });
                    }
                    instance.dispatchEvent({ type: 'click' });
                    dropInstance(instance);
                    fired += 1;
                }
            });
            return { ok: true, fired: fired };
        } catch (error) {
            return { ok: false, error: String((error && error.message) || error) };
        }
    };
})();
