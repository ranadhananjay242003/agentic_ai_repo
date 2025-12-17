FROM node:20-alpine

WORKDIR /app

# 1. Install dependencies
COPY package*.json ./
RUN npm install

# --- FIX START: Accept Keys for Build Step ---
# Define arguments that can be passed during build
ARG NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY
ARG CLERK_SECRET_KEY

# Set them as environment variables so Next.js can see them during 'npm run build'
ENV NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=$NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY
ENV CLERK_SECRET_KEY=$CLERK_SECRET_KEY
# --- FIX END ---

# 2. Copy source code
COPY . .

# 3. Build for production (Now it has the keys!)
RUN npm run build

# 4. Start the application
CMD ["npm", "start"]