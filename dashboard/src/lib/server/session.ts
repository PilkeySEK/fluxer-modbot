export interface User {
	id: string;
	username: string;
	discriminator: string;
	global_name: string | null;
	avatar: string | null;
	avatar_color: number | null;
	flags: number;
	// is_staff: boolean,
	email: string | null;
	bio: string | null;
	pronouns: string | null;
}

const sessions = new Map<string, User>();
export const saveSession = (id: string, user: User) => sessions.set(id, user);
export const getUserFromSession = (id: string) => sessions.get(id) ?? null;
