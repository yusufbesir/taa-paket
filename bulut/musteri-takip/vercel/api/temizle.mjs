// Günlük zamanlanmış görev (vercel.json crons): silinme zamanı geçen kayıtları siler.
import { temizle } from "../lib/depo.mjs";

export default {
  async fetch(istek) {
    if (process.env.CRON_SECRET && istek.headers.get("authorization") !== `Bearer ${process.env.CRON_SECRET}`) {
      return new Response("yetkisiz", { status: 401 });
    }
    return Response.json({ silinen: await temizle() });
  },
};
