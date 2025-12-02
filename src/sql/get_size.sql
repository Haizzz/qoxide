SELECT
    state,
    COUNT(1) AS count
FROM messages
GROUP BY state;
