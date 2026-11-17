from argus.backend.util.config import Config
'\nIntegration tests for Event Similarity Processor V2\n\nThese tests verify the complete flow from event creation to embedding storage.\n'
from dataclasses import asdict
from datetime import datetime, timedelta, UTC
from uuid import UUID, uuid4
from argus.backend.models.argus_ai import SCTErrorEventEmbedding, SCTCriticalEventEmbedding
from argus.backend.models.web import ArgusTest
from argus.backend.plugins.sct.testrun import SCTEvent, SCTEventSeverity, SCTUnprocessedEvent
from argus.backend.service.client_service import ClientService
from argus.backend.plugins.sct.service import SCTService
from argus.backend.service.testrun import TestRunService
from argus.backend.tests.conftest import get_fake_test_run
from argus.common.sct_types import RawEventPayload
from argusAI.event_similarity_processor_v2 import EventSimilarityProcessorV2

def test_event_to_embedding_flow_should_create_embedding_for_error_event(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, *, test_request):
    """Test that ERROR event creates unprocessed entry and processor generates embedding in ERROR table"""
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    event_data: RawEventPayload = {'duration': 30.0, 'event_type': 'DatabaseEvent', 'known_issue': None, 'message': 'Test error message - this is a sample error', 'nemesis_name': None, 'nemesis_status': None, 'node': 'test-node-1', 'received_timestamp': None, 'run_id': run_req.run_id, 'severity': SCTEventSeverity.ERROR.value, 'target_node': None, 'ts': datetime.now(tz=UTC).timestamp()}
    sct_service.submit_event(str(run_req.run_id), event_data)
    events = SCTEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all()
    assert len(list(events)) == 1, 'Event should be created in SCTEvent table'
    unprocessed_events = SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all()
    unprocessed_list = list(unprocessed_events)
    assert len(unprocessed_list) == 1, 'Unprocessed event should be created'
    processor = embedding_processor
    max_attempts = 10
    for attempt in range(max_attempts):
        processor._process_batch()
        remaining = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all())
        if len(remaining) == 0:
            break
    else:
        raise AssertionError(f"Failed to process this test's event after {max_attempts} attempts")
    embeddings = SCTErrorEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all()
    embedding_list = list(embeddings)
    assert len(embedding_list) == 1, 'Embedding should be stored in ERROR table'
    assert len(embedding_list[0].embedding) > 0, 'Embedding should have values'
    critical_embeddings = SCTCriticalEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all()
    assert len(list(critical_embeddings)) == 0, 'No embedding should be in CRITICAL table'
    unprocessed_events_after = SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all()
    assert len(list(unprocessed_events_after)) == 0, 'Unprocessed event should be removed'

def test_event_to_embedding_flow_should_create_embedding_for_critical_event(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, *, test_request):
    """Test that CRITICAL event creates unprocessed entry and processor generates embedding in CRITICAL table"""
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    event_data: RawEventPayload = {'message': 'Critical error - node failure detected', 'run_id': run_req.run_id, 'severity': SCTEventSeverity.CRITICAL.value, 'ts': datetime.now(tz=UTC).timestamp(), 'event_type': 'NodeFailure'}
    sct_service.submit_event(str(run_req.run_id), event_data)
    unprocessed_events = SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.CRITICAL.value).all()
    assert len(list(unprocessed_events)) == 1, 'Unprocessed event should be created'
    processor = embedding_processor
    max_attempts = 10
    for attempt in range(max_attempts):
        processor._process_batch()
        remaining = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.CRITICAL.value).all())
        if len(remaining) == 0:
            break
    else:
        raise AssertionError(f"Failed to process this test's event after {max_attempts} attempts")
    embeddings = SCTCriticalEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all()
    embedding_list = list(embeddings)
    assert len(embedding_list) == 1, 'Embedding should be stored in CRITICAL table'
    assert len(embedding_list[0].embedding) > 0, 'Embedding should have values'
    error_embeddings = SCTErrorEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all()
    assert len(list(error_embeddings)) == 0, 'No embedding should be in ERROR table'

def test_event_to_embedding_flow_should_not_create_unprocessed_for_warning_event(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, *, test_request):
    """Test that WARNING events do NOT create unprocessed entries"""
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    event_data: RawEventPayload = {'message': 'Warning - high latency detected', 'run_id': run_req.run_id, 'severity': SCTEventSeverity.WARNING.value, 'ts': datetime.now(tz=UTC).timestamp(), 'event_type': 'PerformanceWarning'}
    sct_service.submit_event(str(run_req.run_id), event_data)
    events = SCTEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.WARNING.value).all()
    assert len(list(events)) == 1, 'Event should be created in SCTEvent table'
    unprocessed_events = SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.WARNING.value).all()
    assert len(list(unprocessed_events)) == 0, 'Unprocessed event should NOT be created for WARNING'

def test_event_to_embedding_flow_should_process_multiple_events_into_separate_tables(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, *, test_request):
    """Test that processor handles multiple events and stores them in correct severity-specific tables"""
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    for i in range(3):
        event_data: RawEventPayload = {'message': f'Error event {i}', 'run_id': run_req.run_id, 'severity': SCTEventSeverity.ERROR.value, 'ts': datetime.now(tz=UTC).timestamp() + i, 'event_type': 'DatabaseEvent'}
        sct_service.submit_event(str(run_req.run_id), event_data)
    for i in range(2):
        event_data: RawEventPayload = {'message': f'Critical event {i}', 'run_id': run_req.run_id, 'severity': SCTEventSeverity.CRITICAL.value, 'ts': datetime.now(tz=UTC).timestamp() + i + 100, 'event_type': 'CriticalFailure'}
        sct_service.submit_event(str(run_req.run_id), event_data)
    all_unprocessed = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id))).all())
    assert len(all_unprocessed) == 5, 'Should have 5 unprocessed events'
    processor = embedding_processor
    max_attempts = 20
    for attempt in range(max_attempts):
        processor._process_batch(batch_size=100)
        remaining = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id))).all())
        if len(remaining) == 0:
            break
    else:
        raise AssertionError(f"Failed to process all of this test's events after {max_attempts} attempts")
    error_embeddings = list(SCTErrorEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all())
    critical_embeddings = list(SCTCriticalEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all())
    assert len(error_embeddings) == 3, 'Should have 3 ERROR embeddings in ERROR table'
    assert len(critical_embeddings) == 2, 'Should have 2 CRITICAL embeddings in CRITICAL table'
    remaining_unprocessed = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id))).all())
    assert len(remaining_unprocessed) == 0, 'All unprocessed events should be removed'

def test_dedup_marks_same_run_twin_despite_large_other_run_population(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, monkeypatch, *, test_request):
    """Regression (ARGUS-171): a same-run near-identical ERROR event is marked as a duplicate even when
    the embedding table holds a large population of unrelated other-run events.

    The old candidate fetch used a global ``ORDER BY embedding ANN OF ? LIMIT 1000``; once the table
    grew past 1000 rows the current run's own twin was crowded out of the top-1000 by other runs and
    never matched. The fix reads the run's partition directly (``WHERE run_id = ?``), so the twin is
    always found regardless of total table size. This test exercises the real partition read against a
    real ScyllaDB with >1000 other-run embeddings present.
    """
    processor = embedding_processor
    query_vector = [1.0] + [0.0] * 383
    monkeypatch.setattr(processor, 'embedding_model', lambda texts: [list(query_vector) for _ in texts])
    keyspace = SCTErrorEventEmbedding.Settings.keyspace
    insert = processor.db.session.prepare(f'INSERT INTO {keyspace}.{SCTErrorEventEmbedding.table_name()} (run_id, ts, embedding) VALUES (?, ?, ?)')
    base_ts = datetime.now(tz=UTC)
    for i in range(1100):
        processor.db.session.execute(insert, (uuid4(), base_ts + timedelta(microseconds=i), list(query_vector)))
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    message = 'NoHostAvailable Unable to complete the operation ConnectionShutdown Bad file descriptor'
    first_ts = datetime.now(tz=UTC).timestamp()
    for offset in (0, 1):
        sct_service.submit_event(str(run_req.run_id), {'message': message, 'run_id': run_req.run_id, 'severity': SCTEventSeverity.ERROR.value, 'ts': first_ts + offset, 'event_type': 'DatabaseEvent'})
    max_attempts = 20
    for _ in range(max_attempts):
        processor._process_batch(batch_size=100)
        remaining = list(SCTUnprocessedEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all())
        if len(remaining) == 0:
            break
    else:
        raise AssertionError(f"Failed to process this test's events after {max_attempts} attempts")
    events = list(SCTEvent.find(run_id=UUID(str(run_req.run_id)), severity=SCTEventSeverity.ERROR.value).all())
    assert len(events) == 2, 'Both ERROR events should exist in SCTEvent'
    canonical = [e for e in events if e.duplicate_id is None]
    duplicates = [e for e in events if e.duplicate_id is not None]
    assert len(canonical) == 1, 'Exactly one event should remain canonical'
    assert len(duplicates) == 1, 'The near-identical twin should be marked as a duplicate'
    assert duplicates[0].duplicate_id == canonical[0].event_id, 'duplicate_id must point at the canonical event'
    run_embeddings = list(SCTErrorEventEmbedding.find(run_id=UUID(str(run_req.run_id))).all())
    assert len(run_embeddings) == 1, 'Only one embedding should be stored for the run (duplicate skipped)'

def test_event_to_embedding_flow_should_handle_processing_errors_gracefully(client_service: ClientService, sct_service: SCTService, testrun_service: TestRunService, fake_test: ArgusTest, embedding_processor: EventSimilarityProcessorV2, *, test_request):
    """Test that processor handles errors gracefully and removes problematic events"""
    run_type, run_req = get_fake_test_run(fake_test)
    client_service.submit_run(run_type, asdict(run_req), request=test_request)
    event_data: RawEventPayload = {'message': 'Test error', 'run_id': run_req.run_id, 'severity': SCTEventSeverity.ERROR.value, 'ts': datetime.now(tz=UTC).timestamp(), 'event_type': 'TestEvent'}
    sct_service.submit_event(str(run_req.run_id), event_data)
    fake_run_id = uuid4()
    fake_unprocessed = SCTUnprocessedEvent()
    fake_unprocessed.run_id = fake_run_id
    fake_unprocessed.severity = SCTEventSeverity.ERROR.value
    fake_unprocessed.ts = datetime.now(tz=UTC)
    fake_unprocessed.save()
    processor = embedding_processor
    processor._process_batch(batch_size=100)
    assert processor.processed_count >= 1, 'Should process at least one event successfully'
    assert processor.error_count >= 1, 'Should have at least one error'
    fake_unprocessed_after = list(SCTUnprocessedEvent.find(run_id=fake_run_id, severity=SCTEventSeverity.ERROR.value).all())
    assert len(fake_unprocessed_after) == 0, 'Problematic unprocessed event should be removed'
