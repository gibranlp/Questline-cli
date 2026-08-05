export function assignmentId(taskId, userIdentity) {
  if (!taskId || !userIdentity) throw new Error('Task and companion identity are required');
  return `${taskId}__${String(userIdentity).toLowerCase()}`;
}

export function assignmentsForTask(assignments, taskId) {
  return [...assignments.values()].filter(a => a.task_id === taskId);
}

export function isAssignedTo(assignments, taskId, userIdentity) {
  if (!userIdentity) return false;
  return assignments.has(assignmentId(taskId, userIdentity));
}

export function assignmentNotification(eventId, assignment, taskTitle = 'A quest') {
  if (!eventId || !assignment?.task_id || !assignment?.user_identity) return null;
  return {
    id: `task_assignment:${eventId}`,
    notification_type: 'task_assignment',
    title: 'Quest assigned',
    content: `${taskTitle} was assigned to you.`,
    target_id: assignment.task_id,
    project_id: assignment.project_id || null,
    read: false,
    created_at: assignment.assigned_at || new Date().toISOString(),
  };
}
