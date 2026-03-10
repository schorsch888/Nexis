//! Benchmarks for context summarization
//!
//! Run with: cargo bench --bench context_summarization

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;

// We need to conditionally compile based on features
#[cfg(feature = "ai-summarizer")]
use nexis_context::{AISummarizer, ContextSummarizer, Message, SummarizerConfig};
#[cfg(feature = "ai-summarizer")]
use nexis_runtime::{AIProvider, GenerateRequest, GenerateResponse, ProviderError};
#[cfg(feature = "ai-summarizer")]
use async_trait::async_trait;

#[cfg(feature = "ai-summarizer")]
#[derive(Debug)]
struct FastMockProvider;

#[cfg(feature = "ai-summarizer")]
#[async_trait]
impl AIProvider for FastMockProvider {
    fn name(&self) -> &'static str {
        "fast-mock"
    }

    async fn generate(
        &self,
        _req: GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError> {
        // Simulate fast AI response
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(GenerateResponse {
            content: "Summarized content".to_string(),
            model: Some("mock".to_string()),
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn generate_stream(
        &self,
        _req: GenerateRequest,
    ) -> Result<nexis_runtime::ProviderStream, ProviderError> {
        unimplemented!()
    }
}

#[cfg(feature = "ai-summarizer")]
fn bench_summarization(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("summarization");
    
    // Test with different message counts
    for count in [5, 10, 20, 50].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        
        let messages: Vec<Message> = (0..*count)
            .map(|i| {
                if i % 2 == 0 {
                    Message::user(format!("User message {} with some content to make it realistic", i))
                } else {
                    Message::assistant(format!("Assistant response {} with detailed information", i))
                }
            })
            .collect();
        
        let provider = Arc::new(FastMockProvider);
        let summarizer = AISummarizer::new(provider, "mock-model");
        
        group.bench_with_input(BenchmarkId::new("ai_summarize", count), &messages, |b, messages| {
            b.to_async(&rt).iter(|| async {
                black_box(summarizer.summarize(messages).await)
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "ai-summarizer")]
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_summarization
}

#[cfg(not(feature = "ai-summarizer"))]
fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("placeholder_no_ai_summarizer_feature", |b| {
        b.iter(|| black_box(1 + 1))
    });
}

#[cfg(not(feature = "ai-summarizer"))]
criterion_group!(benches, bench_placeholder);

criterion_main!(benches);
