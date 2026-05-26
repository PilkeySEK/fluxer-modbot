import z from 'zod';
import { pool } from './database';
import type { QueryResult } from 'pg';

export const OauthAccessTokenResponseSchema = z.object({
	access_token: z.string(),
	token_type: z.string(),
	expires_in: z.number(),
	refresh_token: z.string(),
	scope: z.string(),
});

export type OAuthAccessTokenResponse = z.infer<typeof OauthAccessTokenResponseSchema>;

export const UserDbSchema = z.object({
	id: z.string(),
	username: z.string(),
	discriminator: z.string(),
	global_name: z.string().nullable(),
	avatar: z.string().nullable(),
	avatar_color: z.number().nullable(),
	flags: z.number(),
	// is_staff: boolean,
	email: z.string().nullable(),
	bio: z.string().nullable(),
	pronouns: z.string().nullable(),
	is_bot_admin: z.boolean(),
	server_data: OauthAccessTokenResponseSchema,
});

export type DbUser = z.infer<typeof UserDbSchema>;

export const UserFluxerApiSchema = z.object({
	id: z.string(),
	username: z.string(),
	discriminator: z.string(),
	global_name: z.string().nullable(),
	avatar: z.string().nullable(),
	avatar_color: z.number().nullable(),
	flags: z.number(),
	// is_staff: boolean,
	email: z.string().nullable(),
	bio: z.string().nullable(),
	pronouns: z.string().nullable(),
});

export async function saveSession(
	sessionId: string,
	expiresAt: Date,
	user: DbUser
): Promise<QueryResult> {
	return await pool.query(
		`INSERT INTO dashboard_oauth_sessions (session_id, expires_at, user_id, data)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (session_id) DO UPDATE
		SET data = EXCLUDED.data`,
		[sessionId, expiresAt, user.id, JSON.stringify(user)]
	);
}
