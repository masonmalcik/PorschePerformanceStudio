import amqp from "amqplib";
import { assertCheckoutTopology } from "./topology.js";

const url = process.env.RABBITMQ_URL;
if (!url) throw new Error("RABBITMQ_URL is required");

const connection = await amqp.connect(url);
try {
  const channel = await connection.createConfirmChannel();
  try {
    await assertCheckoutTopology(channel);
    await channel.waitForConfirms();
    console.log("RabbitMQ checkout topology initialized");
  } finally {
    await channel.close();
  }
} finally {
  await connection.close();
}
