# ==============================================================================
# Stage 1: Build SvelteKit 3 application using Node 26 Alpine
# ==============================================================================
FROM node:26-alpine AS builder
WORKDIR /app

# Install pnpm 9 (matching lockfileVersion 9.0)
RUN npm install -g pnpm@9

# Install all dependencies (including devDependencies required for Vite/Svelte build)
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

# Copy application source code
COPY . .

# Build production bundle using adapter-node
RUN pnpm run build

# ==============================================================================
# Stage 2: Minimal production runtime with Node 26 Alpine
# ==============================================================================
FROM node:26-alpine AS runner
WORKDIR /app

RUN npm install -g pnpm@9

# Install only production dependencies
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --prod --frozen-lockfile

# Copy built application output from builder stage
COPY --from=builder /app/build ./build

ENV PORT=3000
ENV HOST=0.0.0.0
ENV NODE_ENV=production

EXPOSE 3000

CMD ["node", "build"]
