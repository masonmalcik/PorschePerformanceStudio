FROM node:22-alpine AS build
WORKDIR /app
COPY packages/messaging /packages/messaging
COPY PPS_OrderService/package*.json ./
RUN npm ci
COPY PPS_OrderService/prisma ./prisma
RUN npx prisma generate
COPY PPS_OrderService/tsconfig*.json ./
COPY PPS_OrderService/src ./src
RUN npm run build

FROM node:22-alpine AS runtime
ENV NODE_ENV=production
WORKDIR /app
COPY packages/messaging /packages/messaging
COPY PPS_OrderService/package*.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY --from=build /app/node_modules/.prisma ./node_modules/.prisma
COPY --from=build /app/dist ./dist
USER node
EXPOSE 3005
CMD ["node", "dist/main.js"]
