require('dotenv').config();
const { Pool } = require('pg');
const axios = require('axios');

const DATABASE_URL = process.env.DATABASE_URL;
const adminPassword = process.env.ADMIN_PASSWORD;

const pool = new Pool({
    connectionString: DATABASE_URL,
});

async function markRoomsCompleted() {
    const query = `
        UPDATE rooms
        SET
            status = 'completed',
            completed_at = NOW()
        WHERE
            status != 'completed'
            AND completed_at IS NULL
            AND created_at < $1
    `;

    const values = ['2025-12-29T00:00:00Z'];

    const result = await pool.query(query, values);
    console.log(`Rooms updated: ${result.rowCount}`);
}


async function getRoomIdsBeforeDate() {
    const query = `
        SELECT id
        FROM rooms
        WHERE created_at < $1
    `;

    const values = ['2025-12-29T00:00:00Z'];

    const result = await pool.query(query, values);
    return result.rows.map(row => row.id);
}

async function auction_room_completion_tasks() {
    const failed_room_ids = [];
    const succeeded_room_ids = [];
    let total_errors = 0;

    console.log("Running post auction completion tasks");

    const room_ids = await getRoomIdsBeforeDate();

    for (const x of room_ids) {
        try {
            const res = await axios.post(
                "http://localhost:4545/admin/auction_completed_tasks_execution",
                {
                    room_id: x,
                    password: adminPassword,
                }
            );

            if (res.status !== 200) {
                failed_room_ids.push(x);
            } else {
                succeeded_room_ids.push(x);
            }
        } catch (error) {
            console.error(`Error for room ${x}`, error.message);
            total_errors++;
        }
    }

    console.log("Total Room IDs:", room_ids.length);
    console.log("Succeeded:", succeeded_room_ids.length);
    console.log("Failed:", failed_room_ids.length);
    console.log("Errored:", total_errors);
}

async function main() {
    try {
        console.log("Executing cleanup tasks");
        await markRoomsCompleted();
        await auction_room_completion_tasks();
        console.log("All tasks completed");
    } catch (err) {
        console.error("Fatal error:", err);
    } finally {
        await pool.end();
    }
}

main();
