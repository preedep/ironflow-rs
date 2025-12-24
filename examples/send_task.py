#!/usr/bin/env python3
"""
Example script to send tasks to IronFlow-Rs via Redis
"""

import redis
import json
import uuid
from datetime import datetime

def create_execute_command_task():
    """Create a simple command execution task"""
    return {
        "id": str(uuid.uuid4()),
        "correlation_id": "example-batch-001",
        "task_type": {
            "type": "execute_command",
            "command": "echo",
            "args": ["Hello from IronFlow-Rs!"],
            "working_dir": None,
            "env_vars": None
        },
        "priority": 0,
        "max_retries": 3,
        "retry_count": 0,
        "callback_url": None,
        "result_queue": "ironflow:results",
        "created_at": datetime.utcnow().isoformat() + "Z",
        "scheduled_at": None,
        "timeout_secs": 60,
        "metadata": {
            "source": "example_script",
            "environment": "development"
        }
    }

def create_file_transfer_task():
    """Create a file transfer task"""
    return {
        "id": str(uuid.uuid4()),
        "correlation_id": "example-batch-002",
        "task_type": {
            "type": "file_transfer",
            "source": {
                "type": "local",
                "path": "/tmp/source.txt"
            },
            "destination": {
                "type": "local",
                "path": "/tmp/destination.txt"
            },
            "options": {
                "overwrite": True,
                "create_dirs": True,
                "preserve_metadata": True,
                "chunk_size": None
            }
        },
        "priority": 0,
        "max_retries": 3,
        "retry_count": 0,
        "callback_url": None,
        "result_queue": "ironflow:results",
        "created_at": datetime.utcnow().isoformat() + "Z",
        "scheduled_at": None,
        "timeout_secs": 300,
        "metadata": {}
    }

def create_composite_task():
    """Create a composite task with multiple steps"""
    return {
        "id": str(uuid.uuid4()),
        "correlation_id": "example-batch-003",
        "task_type": {
            "type": "composite",
            "tasks": [
                {
                    "type": "execute_command",
                    "command": "echo",
                    "args": ["Step 1: Starting process"],
                    "working_dir": None,
                    "env_vars": None
                },
                {
                    "type": "execute_command",
                    "command": "echo",
                    "args": ["Step 2: Processing data"],
                    "working_dir": None,
                    "env_vars": None
                },
                {
                    "type": "execute_command",
                    "command": "echo",
                    "args": ["Step 3: Completed"],
                    "working_dir": None,
                    "env_vars": None
                }
            ],
            "stop_on_error": True
        },
        "priority": 0,
        "max_retries": 3,
        "retry_count": 0,
        "callback_url": None,
        "result_queue": "ironflow:results",
        "created_at": datetime.utcnow().isoformat() + "Z",
        "scheduled_at": None,
        "timeout_secs": 600,
        "metadata": {}
    }

def send_task_to_redis(task, redis_url="redis://localhost:6379", queue_name="ironflow:tasks"):
    """Send a task to Redis queue"""
    try:
        r = redis.from_url(redis_url)
        task_json = json.dumps(task)
        r.lpush(queue_name, task_json)
        print(f"✓ Task sent successfully: {task['id']}")
        print(f"  Type: {task['task_type']['type']}")
        print(f"  Correlation ID: {task.get('correlation_id', 'N/A')}")
        return True
    except Exception as e:
        print(f"✗ Failed to send task: {e}")
        return False

def main():
    """Main function"""
    print("IronFlow-Rs Task Sender Example\n")
    print("=" * 50)
    
    # Send execute command task
    print("\n1. Sending Execute Command Task...")
    task1 = create_execute_command_task()
    send_task_to_redis(task1)
    
    # Send file transfer task
    print("\n2. Sending File Transfer Task...")
    task2 = create_file_transfer_task()
    send_task_to_redis(task2)
    
    # Send composite task
    print("\n3. Sending Composite Task...")
    task3 = create_composite_task()
    send_task_to_redis(task3)
    
    print("\n" + "=" * 50)
    print("\nAll tasks sent! Check the worker logs for results.")
    print("Results will be available in the 'ironflow:results' queue.")

if __name__ == "__main__":
    main()
