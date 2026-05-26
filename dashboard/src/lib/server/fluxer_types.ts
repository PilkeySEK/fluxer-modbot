import z from 'zod';

const SNOWFLAKE_REGEX = /^(0|[1-9][0-9]*)$/;
const UNSIGNED_INT64_STRING_REGEX = /^\d+$/;

export const SnowflakeStringType = z
	.string()
	.regex(SNOWFLAKE_REGEX)
	.describe('fluxer:SnowflakeStringType');

export const Int32Type = z.number().int().min(0).max(2147483647).describe('fluxer:Int32Type');

export const PermissionStringType = z.string().regex(UNSIGNED_INT64_STRING_REGEX);

// Fields that we will likely never need are commented out here
export const GuildResponseSchema = z.object({
	id: SnowflakeStringType,
	name: z.string().describe('The name of the guild'),
	icon: z.string().nullish().describe('The hash of the guild icon'),
	banner: z.string().nullish().describe('The hash of the guild banner'),
	banner_width: Int32Type.nullish().describe('The width of the guild banner in pixels'),
	banner_height: Int32Type.nullish().describe('The height of the guild banner in pixels'),
	splash: z.string().nullish().describe('The hash of the guild splash screen'),
	splash_width: Int32Type.nullish().describe('The width of the guild splash in pixels'),
	splash_height: Int32Type.nullish().describe('The height of the guild splash in pixels'),
	// splash_card_alignment: SplashCardAlignmentSchema.describe('The alignment of the splash card'),
	embed_splash: z.string().nullish().describe('The hash of the embedded invite splash'),
	embed_splash_width: Int32Type.nullish().describe(
		'The width of the embedded invite splash in pixels'
	),
	embed_splash_height: Int32Type.nullish().describe(
		'The height of the embedded invite splash in pixels'
	),
	vanity_url_code: z.string().nullish().describe('The vanity URL code for the guild'),
	owner_id: SnowflakeStringType.describe('The ID of the guild owner'),
	system_channel_id: SnowflakeStringType.nullish().describe(
		'The ID of the channel where system messages are sent'
	),
	// system_channel_flags: createBitflagInt32Type(
	// 	SystemChannelFlags,
	// 	SystemChannelFlagsDescriptions,
	// 	'System channel message flags',
	// 	'SystemChannelFlags',
	// ),
	// rules_channel_id: SnowflakeStringType.nullish().describe('The ID of the rules channel'),
	// afk_channel_id: SnowflakeStringType.nullish().describe('The ID of the AFK voice channel'),
	// afk_timeout: Int32Type.describe('AFK timeout in seconds before moving users to the AFK channel'),
	// features: GuildFeatureListSchema,
	// verification_level: withFieldDescription(
	// 	GuildVerificationLevelSchema,
	// 	'Required verification level for members to participate',
	// ),
	// mfa_level: withFieldDescription(GuildMFALevelSchema, 'Required MFA level for moderation actions'),
	// nsfw_level: withFieldDescription(NSFWLevelSchema, 'The NSFW level of the guild'),
	// explicit_content_filter: withFieldDescription(
	// 	GuildExplicitContentFilterSchema,
	// 	'Level of content filtering for explicit media',
	// ),
	// default_message_notifications: withFieldDescription(
	// 	DefaultMessageNotificationsSchema,
	// 	'Default notification level for new members',
	// ),
	// disabled_operations: createBitflagInt32Type(
	// 	GuildOperations,
	// 	GuildOperationsDescriptions,
	// 	'Bitfield of disabled operations in the guild',
	// 	'GuildOperations',
	// ),
	// message_history_cutoff: z.iso
	// 	.datetime()
	// 	.nullish()
	// 	.describe(
	// 		'ISO8601 timestamp controlling how far back members without Read Message History can access messages. When null, no historical access is allowed.',
	// 	),
	permissions: PermissionStringType.describe(
		'The current user permissions in this guild'
	).nullish(),
});

export type GuildResponse = z.infer<typeof GuildResponseSchema>;
