const CACHE_NAME = "sgleam-v1";
const ASSETS = [
    ".",
    "index.html",
    "player.html",
    "sgleam.wasm",
    "manifest.json",
    "icon-192.svg",
    "icon-512.svg",
];

self.addEventListener("install", (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => cache.addAll(ASSETS)),
    );
});

self.addEventListener("activate", (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(
                keys
                    .filter((key) => key !== CACHE_NAME)
                    .map((key) => caches.delete(key)),
            )
        ),
    );
});

self.addEventListener("fetch", (event) => {
    event.respondWith(
        caches.match(event.request).then((cached) => {
            // Network first, fall back to cache
            return fetch(event.request)
                .then((response) => {
                    const clone = response.clone();
                    caches.open(CACHE_NAME).then((cache) =>
                        cache.put(event.request, clone)
                    );
                    return response;
                })
                .catch(() => cached);
        }),
    );
});
