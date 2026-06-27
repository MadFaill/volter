/** @type {import('tailwindcss').Config} */
// Токены из ui-concept.md §3 (светлая «дневной прибор» тема).
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: "#FBFBFC",
        surface: "#FFFFFF",
        ink: "#14181F",
        muted: "#5B6573",
        line: "#E7E9EE",
        volt: "#2E5BFF",
        "volt-wash": "#EEF2FF",
        pass: "#1F9D6B",
        run: "#B8780A",
        fail: "#D14343",
      },
      fontFamily: {
        sans: ['"Hanken Grotesk"', "system-ui", "sans-serif"],
        mono: ['"IBM Plex Mono"', '"JetBrains Mono"', "ui-monospace", "monospace"],
      },
      fontSize: {
        micro: ["11px", { lineHeight: "16px", letterSpacing: "0.04em" }],
        small: ["13px", { lineHeight: "18px" }],
        body: ["15px", { lineHeight: "24px" }],
        title: ["17px", { lineHeight: "24px" }],
        display: ["24px", { lineHeight: "30px" }],
        "mono-data": ["12px", { lineHeight: "18px" }],
        "mono-code": ["13px", { lineHeight: "20px" }],
      },
      borderRadius: { sm: "6px", md: "10px", pill: "999px" },
      boxShadow: {
        card: "0 1px 2px rgba(20,24,31,.04), 0 1px 1px rgba(20,24,31,.03)",
      },
      maxWidth: { thread: "720px" },
    },
  },
  plugins: [],
};
