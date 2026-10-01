let current = $state(location.hash.replace(/^#/, "") || "/");

function onChange() {
  current = location.hash.replace(/^#/, "") || "/";
}

if (typeof window !== "undefined") {
  window.addEventListener("hashchange", onChange);
}

export function getPath(): string {
  return current;
}

export function navigate(to: string) {
  if (location.hash === `#${to}`) {
    onChange();
  } else {
    location.hash = to;
  }
}