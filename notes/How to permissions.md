From Sapphire's docs (https://docs.sapph.xyz/#/generalsettings?id=manager-roles):

> Manager Roles grant users the ability to fully manage Sapphire and its features. Users with a manager role are able to view the dashboard and change all settings.
>
> [...]
>
> Roles with `Manage Server` or `Administrator` permissions are automatically added and cannot be removed.

There are also advanced permissions which are _only_ related to the dashboard(-tabs) which they can access and/or modify. The worker doesn't need to care about these, only the API and svelte dashboard.

Commands in Sapphire can be configured to be allowed for certain roles, or the user needs specific permissions, or they are allowed in certain channels. All of these stack, meaning all requirements must be satisfied in order for the command to be allowed. I assume administrators can always bypass these requirements.

... It seems like roles and permissions do not stack and instead are "ORed". So, a user can have _either_ the correct permissions or one of the specified roles. The channel still needs to be correct regardless.