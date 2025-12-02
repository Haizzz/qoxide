WITH selected_message AS (
    SELECT id FROM messages 
    WHERE state = 'PENDING' 
    LIMIT 1
)
UPDATE messages
SET state = 'RESERVED'
WHERE id IN (SELECT id FROM selected_message)
RETURNING 
    messages.id,
    (SELECT data FROM payloads WHERE id = messages.payload_id) AS data;