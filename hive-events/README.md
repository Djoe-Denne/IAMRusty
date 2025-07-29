# Hive Events

Domain events for the Hive organization management service.

## Overview

This crate contains all domain events that are published by the Hive service for inter-service communication. Events are primarily consumed by the **Telegraph** service for sending email notifications and by other services for integration purposes.

## Event Categories

### Organization Events
- `OrganizationCreatedEvent` - Published when a new organization is created
- `OrganizationUpdatedEvent` - Published when organization details are modified
- `OrganizationDeletedEvent` - Published when an organization is deleted

### Member Events
- `MemberInvitedEvent` - Published when a user is invited to join (triggers email)
- `MemberJoinedEvent` - Published when a user accepts an invitation
- `MemberRemovedEvent` - Published when a member is removed (triggers email)

### Invitation Events
- `InvitationCreatedEvent` - Published when an invitation is created (triggers email)
- `InvitationAcceptedEvent` - Published when an invitation is accepted (triggers email)
- `InvitationExpiredEvent` - Published when an invitation expires (triggers email)

### External Integration Events
- `ExternalLinkCreatedEvent` - Published when an external provider is linked
- `SyncJobStartedEvent` - Published when a sync job begins
- `SyncJobCompletedEvent` - Published when a sync job finishes (triggers email on failure)

## Queue Routing

Events are routed to different SQS queues based on their purpose:

### `organization-events`
Organizational state changes:
- Organization lifecycle events
- Member joined events  
- External link creation

### `notification-events` 
Events that trigger email notifications via Telegraph:
- All invitation-related events
- Member removal notifications
- Sync job failures

### `sync-events`
Sync job monitoring and tracking:
- Job started/completed events
- Performance metrics

## Usage

```rust
use hive_events::{
    MemberInvitedEvent, 
    queues::NOTIFICATION_EVENTS,
    event_types::MEMBER_INVITED
};

// Create an event
let event = MemberInvitedEvent {
    organization_id: org_id,
    organization_name: "Acme Corp".to_string(),
    invitation_id: invite_id,
    email: "user@example.com".to_string(),
    role_name: "Developer".to_string(),
    invited_by_user_id: inviter_id,
    invitation_token: "secure_token".to_string(),
    expires_at: Utc::now() + Duration::days(7),
    message: Some("Welcome to the team!".to_string()),
};

// Publish to Telegraph for email notification
publisher.publish(
    NOTIFICATION_EVENTS,
    MEMBER_INVITED,
    serde_json::to_value(&event)?
).await?;
```

## Integration with Telegraph

The Telegraph service consumes events from the `notification-events` queue and sends appropriate email notifications using predefined templates. Each event type corresponds to a specific email template and communication flow.

## Serialization

All events implement `Serialize` and `Deserialize` for JSON serialization, making them compatible with SQS message formats and inter-service communication protocols. 