import { DATABASE_URL } from '$env/static/private';
import { Pool } from 'pg';
import { UserDbSchema, type DbUser } from './session';

export const pool = new Pool({
	connectionString: DATABASE_URL,
});

let lastCleanup = 0;
const SESSION_CLEANUP_INTERVAL = 1000 * 60 * 5; // 5 minutes

export function cleanupSessions() {
	const now = Date.now();

	if (now - lastCleanup > SESSION_CLEANUP_INTERVAL) {
		lastCleanup = now;

		pool
			.query(
				`DELETE FROM dashboard_oauth_sessions
                WHERE expires_at < NOW()`
			)
			.catch(console.error);
	}
}

export async function getUserData(sessionId: string): Promise<DbUser | null> {
	const res = await pool.query(
		`SELECT data FROM dashboard_oauth_sessions
		WHERE session_id = $1`,
		[sessionId]
	);
	if (res.rowCount === 0) {
		return null;
	}

	const user = UserDbSchema.parse(res.rows[0].data);
	return user;
}

export async function getUserDataById(user_id: string): Promise<DbUser | null> {
	const res = await pool.query(
		`SELECT data FROM dashboard_oauth_sessions
		WHERE user_id = $1
		LIMIT 1`,
		[user_id]
	);
	if (res.rowCount === 0) {
		return null;
	}

	const user = UserDbSchema.parse(res.rows[0].data);
	return user;
}
