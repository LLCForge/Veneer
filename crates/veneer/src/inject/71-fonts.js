// Fonts: families that are neither in the profile, generic, nor loaded as web fonts are renamed to a
// family that cannot exist, so they fall back exactly as on a machine without them. Getters strip
// the rename again. document.fonts.check() is left alone: Chromium answers true for any family
// that is not a web font, installed or not.
const part = (P, H) => {
  const allowed = new Set(P.fonts.map((f) => f.toLowerCase()));
  const GENERIC = new Set([
    'serif', 'sans-serif', 'monospace', 'cursive', 'fantasy', 'system-ui', 'ui-serif', 'ui-sans-serif',
    'ui-monospace', 'ui-rounded', 'emoji', 'math', 'fangsong', '-apple-system', 'blinkmacsystemfont',
    'inherit', 'initial', 'unset', 'revert', 'revert-layer',
  ]);
  const MARK = ' Vnrhiddenfont';
  const fontSet = () => (globalThis.document ? globalThis.document.fonts : globalThis.fonts);
  const isWebFont = (name) => {
    try {
      for (const face of fontSet()) {
        if (face.family.replace(/^["']|["']$/g, '').toLowerCase() === name) return true;
      }
    } catch {
      // No FontFaceSet in this scope.
    }
    return false;
  };
  const split = (list) => {
    const out = [];
    let cur = '';
    let quote = null;
    for (const ch of list) {
      if (quote) {
        cur += ch;
        if (ch === quote) quote = null;
      } else if (ch === '"' || ch === "'") {
        quote = ch;
        cur += ch;
      } else if (ch === ',') {
        out.push(cur);
        cur = '';
      } else cur += ch;
    }
    out.push(cur);
    return out;
  };
  const hideFamily = (token) => {
    const t = token.trim();
    if (!t) return token;
    const quoted = /^(["']).*\1$/s.test(t);
    const name = (quoted ? t.slice(1, -1) : t.replace(/\s+/g, ' ')).toLowerCase();
    if (name.includes(MARK.trim().toLowerCase()) || GENERIC.has(name) || allowed.has(name) || isWebFont(name)) return token;
    return quoted ? t.slice(0, -1) + MARK + t.slice(-1) : t + MARK;
  };
  const families = (value) => (typeof value === 'string' && value.length < 4096 ? split(value).map(hideFamily).join(',') : value);
  const SHORTHAND = /^(.*?(?:[\d.]+(?:px|pt|pc|em|rem|ex|ch|in|cm|mm|q|vw|vh|vmin|vmax|%)|xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large|larger|smaller)(?:\s*\/\s*\S+)?\s+)(\S.*)$/is;
  const shorthand = (value) => {
    if (typeof value !== 'string' || value.length >= 4096) return value;
    const m = SHORTHAND.exec(value);
    return m ? m[1] + families(m[2]) : value;
  };
  // Serializers quote a renamed single-word family; the original would not have been quoted.
  const unmark = (value) =>
    typeof value === 'string' && value.includes(MARK)
      ? value
          .replace(new RegExp(`(["'])([^"']*?)${MARK}\\1`, 'g'), (m, q, name) => (/^[A-Za-z_][\w-]*$/.test(name) ? name : q + name + q))
          .split(MARK)
          .join('')
      : value;

  const patchAccessor = (owner, key, rewrite) => {
    const d = owner && Object.getOwnPropertyDescriptor(owner, key);
    if (!d || !d.get || !d.set) return;
    const { get: nativeGet, set: nativeSet } = d;
    const holder = {
      get [key]() {
        return unmark(nativeGet.call(this));
      },
      set [key](value) {
        nativeSet.call(this, rewrite(value));
      },
    };
    const h = Object.getOwnPropertyDescriptor(holder, key);
    Object.defineProperty(owner, key, { ...d, get: H.disguise(h.get, `get ${key}`), set: H.disguise(h.set, `set ${key}`) });
  };

  for (const Ctx of [globalThis.CanvasRenderingContext2D, globalThis.OffscreenCanvasRenderingContext2D]) {
    if (Ctx) patchAccessor(Ctx.prototype, 'font', shorthand);
  }

  if (globalThis.document && globalThis.CSSStyleDeclaration) {
    const style = globalThis.document.createElement('div').style;
    const [familyOwner] = H.findDescriptor(style, 'fontFamily');
    patchAccessor(familyOwner, 'fontFamily', families);
    const [fontOwner] = H.findDescriptor(style, 'font');
    patchAccessor(fontOwner, 'font', shorthand);
    const proto = CSSStyleDeclaration.prototype;
    H.spoofMethod(proto, 'setProperty', (native) => ({
      setProperty(property, value) {
        const args = [...arguments];
        const name = typeof property === 'string' ? property.trim().toLowerCase() : '';
        if (name === 'font-family') args[1] = families(value);
        else if (name === 'font') args[1] = shorthand(value);
        return native.apply(this, args);
      },
    }).setProperty);
    H.spoofMethod(proto, 'getPropertyValue', (native) => ({
      getPropertyValue(property) {
        return unmark(native.apply(this, arguments));
      },
    }).getPropertyValue);
    H.wrapGetter(proto, 'cssText', unmark);
  }

  // FontFace with local() sources finds installed fonts; hide the ones the profile does not have.
  const NativeFontFace = globalThis.FontFace;
  if (NativeFontFace) {
    const hideLocal = (src) =>
      typeof src === 'string' && src.length < 65536
        ? src.replace(/local\(\s*(["']?)([^"')]*?)\1\s*\)/gi, (m, q, name) =>
            allowed.has(name.trim().toLowerCase()) ? m : `local(${q || '"'}${name}${MARK}${q || '"'})`)
        : src;
    const Wrapped = new Proxy(NativeFontFace, {
      construct(target, args, newTarget) {
        const list = [...args];
        if (list.length > 1) list[1] = hideLocal(list[1]);
        return Reflect.construct(target, list, newTarget === Wrapped ? target : newTarget);
      },
    });
    H.disguise(Wrapped, 'FontFace');
    const ctor = Object.getOwnPropertyDescriptor(NativeFontFace.prototype, 'constructor');
    if (ctor) Object.defineProperty(NativeFontFace.prototype, 'constructor', { ...ctor, value: Wrapped });
    const global = Object.getOwnPropertyDescriptor(globalThis, 'FontFace');
    if (global) Object.defineProperty(globalThis, 'FontFace', { ...global, value: Wrapped });
  }
};
part(P, veneerH, false);
veneerParts.push(part);
