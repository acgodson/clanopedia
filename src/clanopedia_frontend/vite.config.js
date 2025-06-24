import { fileURLToPath, URL } from "url";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";
import environment from "vite-plugin-environment";
import path from "path";
import fs from "fs";

export default defineConfig(({ mode }) => {
  // Get the current file's directory
  const __dirname = path.dirname(fileURLToPath(import.meta.url));

  // Go up two levels to reach the project root (where dfx.json is located)
  const projectRoot = path.resolve(__dirname, "../../");

  // Load env file from the project root
  const env = loadEnv(mode, projectRoot, "");

  // Determine the actual mode based on DFX_NETWORK (like dfx.json does)
  const dfxNetwork = env.DFX_NETWORK || "ic";
  const actualMode = dfxNetwork === "local" ? "development" : "production";

  console.log("=== ENV DEBUG INFO ===");
  console.log("Project root:", projectRoot);
  console.log("Vite mode:", mode);
  console.log("DFX_NETWORK:", dfxNetwork);
  console.log("Actual mode (based on DFX_NETWORK):", actualMode);
  console.log("Environment variables loaded:", {
    DFX_NETWORK: env.DFX_NETWORK,
    CANISTER_ID_INTERNET_IDENTITY: env.CANISTER_ID_INTERNET_IDENTITY,
    CANISTER_ID_CLANOPEDIA_BACKEND: env.CANISTER_ID_CLANOPEDIA_BACKEND,
    CANISTER_ID_BLUEBAND_RUST: env.CANISTER_ID_BLUEBAND_RUST,
    CANISTER_ID_CLANOPEDIA_FRONTEND: env.CANISTER_ID_CLANOPEDIA_FRONTEND,
  });
  console.log("=== END DEBUG INFO ===");

  return {
    define: {
      // Use the actual mode based on DFX_NETWORK
      "process.env.NODE_ENV": JSON.stringify(actualMode),
      "import.meta.env.DEV": JSON.stringify(actualMode === "development"),
      "import.meta.env.PROD": JSON.stringify(actualMode === "production"),
      "import.meta.env.MODE": JSON.stringify(actualMode),

      // Expose DFX-related variables
      "process.env.DFX_NETWORK": JSON.stringify(env.DFX_NETWORK),
      "import.meta.env.DFX_NETWORK": JSON.stringify(env.DFX_NETWORK),

      // Expose canister IDs
      "process.env.CANISTER_ID_INTERNET_IDENTITY": JSON.stringify(
        env.CANISTER_ID_INTERNET_IDENTITY
      ),
      "process.env.CANISTER_ID_CLANOPEDIA_BACKEND": JSON.stringify(
        env.CANISTER_ID_CLANOPEDIA_BACKEND
      ),
      "process.env.CANISTER_ID_BLUEBAND_RUST": JSON.stringify(
        env.CANISTER_ID_BLUEBAND_RUST
      ),
      "process.env.CANISTER_ID_CLANOPEDIA_FRONTEND": JSON.stringify(
        env.CANISTER_ID_CLANOPEDIA_FRONTEND
      ),

      // Also expose them on import.meta.env for consistency
      "import.meta.env.CANISTER_ID_INTERNET_IDENTITY": JSON.stringify(
        env.CANISTER_ID_INTERNET_IDENTITY
      ),
      "import.meta.env.CANISTER_ID_CLANOPEDIA_BACKEND": JSON.stringify(
        env.CANISTER_ID_CLANOPEDIA_BACKEND
      ),
      "import.meta.env.CANISTER_ID_BLUEBAND_RUST": JSON.stringify(
        env.CANISTER_ID_BLUEBAND_RUST
      ),
      "import.meta.env.CANISTER_ID_CLANOPEDIA_FRONTEND": JSON.stringify(
        env.CANISTER_ID_CLANOPEDIA_FRONTEND
      ),
    },
    build: {
      emptyOutDir: true,
    },
    optimizeDeps: {
      esbuildOptions: {
        define: {
          global: "globalThis",
        },
      },
    },
    server: {
      proxy: {
        "/api": {
          target: "http://127.0.0.1:4943",
          changeOrigin: true,
        },
      },
    },
    plugins: [
      react(),
      environment({
        prefix: ["CANISTER_", "DFX_"],
        defineOn: "import.meta.env",
      }),
    ],
    resolve: {
      alias: [
        {
          find: "declarations",
          replacement: fileURLToPath(
            new URL("../declarations", import.meta.url)
          ),
        },
      ],
      dedupe: ["@dfinity/agent"],
    },
  };
});
