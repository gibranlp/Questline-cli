import test from 'node:test';
import assert from 'node:assert/strict';
import {
  assignmentId,
  assignmentNotification,
  assignmentsForTask,
  isAssignedTo,
} from '../src/lib/collaboration.js';

test('assignment IDs stay byte-compatible with the CLI compound contract', () => {
  assert.equal(assignmentId('task-1', 'AABB'), 'task-1__aabb');
});

test('assignment lookup supports multiple companions without replacing peers', () => {
  const assignments = new Map([
    ['task-1__alice', { task_id: 'task-1', user_identity: 'alice' }],
    ['task-1__bob', { task_id: 'task-1', user_identity: 'bob' }],
    ['task-2__alice', { task_id: 'task-2', user_identity: 'alice' }],
  ]);
  assert.equal(assignmentsForTask(assignments, 'task-1').length, 2);
  assert.equal(isAssignedTo(assignments, 'task-1', 'ALICE'), true);
  assert.equal(isAssignedTo(assignments, 'task-1', 'carol'), false);
});

test('assignment notification ID is deterministic for retry deduplication', () => {
  const payload = { task_id: 'task-1', user_identity: 'alice', project_id: 'project-1' };
  const first = assignmentNotification('event-1', payload, 'Ship Questline');
  const retry = assignmentNotification('event-1', payload, 'Ship Questline');
  assert.equal(first.id, 'task_assignment:event-1');
  assert.equal(retry.id, first.id);
  assert.equal(first.target_id, 'task-1');
});
