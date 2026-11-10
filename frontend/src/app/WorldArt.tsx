export function WorldArt() {
  return (
    <svg
      className="world-art"
      aria-hidden="true"
      viewBox="0 0 520 380"
      preserveAspectRatio="xMidYMid slice"
    >
      <defs>
        <linearGradient id="world-top" x2="1" y2="1" gradientUnits="objectBoundingBox">
          <stop stopColor="#b1ffbf" />
          <stop offset="1" stopColor="#51bb9d" />
        </linearGradient>
        <linearGradient id="world-side" x2="0" y2="1" gradientUnits="objectBoundingBox">
          <stop stopColor="#705aaf" />
          <stop offset="1" stopColor="#25264e" />
        </linearGradient>
        <linearGradient id="world-front" x2="1" y2="1" gradientUnits="objectBoundingBox">
          <stop stopColor="#433471" />
          <stop offset="1" stopColor="#1b1b40" />
        </linearGradient>
        <radialGradient id="world-glow">
          <stop stopColor="#a699ff" stopOpacity=".42" />
          <stop offset="1" stopColor="#7d67ef" stopOpacity="0" />
        </radialGradient>
        <pattern id="world-stars" width="30" height="30" patternUnits="userSpaceOnUse">
          <circle cx="2" cy="2" r="1" fill="#e9e5ff" opacity=".15" />
        </pattern>
      </defs>
      <rect x="65" y="8" width="455" height="360" fill="url(#world-stars)" />
      <ellipse cx="322" cy="179" rx="210" ry="156" fill="url(#world-glow)" />
      <g fill="none" stroke="#c7baff" strokeOpacity=".2">
        <ellipse cx="319" cy="207" rx="193" ry="96" transform="rotate(-18 319 207)" />
        <ellipse
          cx="319"
          cy="207"
          rx="211"
          ry="115"
          transform="rotate(-18 319 207)"
          strokeDasharray="2 8"
        />
        <path d="M100 92h46m-23-23v46m315 167h27m-13.5-13.5v27" />
      </g>
      <g fill="#e4dbff">
        <circle cx="158" cy="228" r="2.5" opacity=".65" />
        <circle cx="432" cy="104" r="3" opacity=".8" />
        <circle cx="490" cy="184" r="2" opacity=".55" />
        <circle cx="227" cy="63" r="2" opacity=".55" />
      </g>
      <g>
        <path d="m306 104 130 69-131 69-130-69z" fill="url(#world-top)" />
        <path
          d="m175 173 130 69v91l-42-22v-17l-26-14v-21l-34-18v-22l-28-14z"
          fill="url(#world-side)"
        />
        <path
          d="m305 242 131-69v43l-29 15v22l-30 16v21l-27 14v6l-45 23z"
          fill="url(#world-front)"
        />
        <path d="m175 173 130 69v24l-31-16v-10l-32-17v7l-34-18v-13l-33-18z" fill="#529c91" />
        <path d="m305 242 131-69v10l-24 13v12l-36 19v-6l-30 16v12l-41 22z" fill="#3b727c" />
        <path d="m306 104 130 69-131 69-130-69z" fill="none" stroke="#b5ffe0" strokeOpacity=".7" />
        <path
          d="m209 228 19 10v12l-19-10zm49 28 26 14v15l-26-14zm16 35 15 8v10l-15-8z"
          fill="#b0a0e6"
          opacity=".27"
        />
        <path d="m322 283 24-13v12l-24 13zm58-38 23-12v9l-23 12z" fill="#ba9ce8" opacity=".22" />
        <path
          d="m306 104 39 21-38 20-39-21zm-66 34 38 20-32 17-39-20zm111 42 35 18-33 17-35-18z"
          fill="#e1ffdb"
          opacity=".25"
        />
      </g>
      <g>
        <path d="m297 155 30 16v-69l-30-16z" fill="#65529c" />
        <path d="m327 171 15-8V94l-15 8z" fill="#30264e" />
        <path d="m319 48 54 29-54 29-54-29z" fill="#d3ffcf" />
        <path d="m265 77 54 29v40l-54-29z" fill="#62b89a" />
        <path d="m319 106 54-29v40l-54 29z" fill="#397f84" />
        <path d="m286 68 16 9-15 8-16-8zm46 3 17 9-13 7-17-9z" fill="#fff" opacity=".31" />
        <path d="m277 96 16 9v9l-16-9zm24 21 12 7v8l-12-6z" fill="#b1f5bb" opacity=".47" />
      </g>
      <g>
        <path d="m152 247 23 12-23 13-24-13z" fill="#b0a1ef" />
        <path d="m128 259 24 13v22l-24-13z" fill="#6f5db0" />
        <path d="m152 272 23-13v22l-23 13z" fill="#41326c" />
        <path d="m458 227 19 10-19 10-19-10z" fill="#90e2c3" />
        <path d="m439 237 19 10v16l-19-10z" fill="#4b9e91" />
        <path d="m458 247 19-10v16l-19 10z" fill="#386a7e" />
      </g>
    </svg>
  );
}
