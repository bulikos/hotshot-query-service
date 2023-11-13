(function() {var implementors = {
"hotshot_query_service":[["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/availability/enum.ResourceId.html\" title=\"enum hotshot_query_service::availability::ResourceId\">ResourceId</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::data_source::ResourceId"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/availability/enum.QueryError.html\" title=\"enum hotshot_query_service::availability::QueryError\">QueryError</a>",1,["hotshot_query_service::availability::data_source::QueryError"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.NotFoundSnafu.html\" title=\"struct hotshot_query_service::availability::NotFoundSnafu\">NotFoundSnafu</a>",1,["hotshot_query_service::availability::data_source::NotFoundSnafu"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.MissingSnafu.html\" title=\"struct hotshot_query_service::availability::MissingSnafu\">MissingSnafu</a>",1,["hotshot_query_service::availability::data_source::MissingSnafu"]],["impl&lt;__T0&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.Snafu.html\" title=\"struct hotshot_query_service::availability::Snafu\">Snafu</a>&lt;__T0&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::data_source::Snafu"]],["impl&lt;Types, I&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.LeafQueryData.html\" title=\"struct hotshot_query_service::availability::LeafQueryData\">LeafQueryData</a>&lt;Types, I&gt;<span class=\"where fmt-newline\">where\n    &lt;I as NodeImplementation&lt;Types&gt;&gt;::Leaf: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::SignatureKey as SignatureKey&gt;::QCType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::LeafQueryData"]],["impl&lt;Types, I&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.InconsistentLeafError.html\" title=\"struct hotshot_query_service::availability::InconsistentLeafError\">InconsistentLeafError</a>&lt;Types, I&gt;<span class=\"where fmt-newline\">where\n    &lt;I as NodeImplementation&lt;Types&gt;&gt;::Leaf: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::SignatureKey as SignatureKey&gt;::QCType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::InconsistentLeafError"]],["impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.BlockQueryData.html\" title=\"struct hotshot_query_service::availability::BlockQueryData\">BlockQueryData</a>&lt;Types&gt;<span class=\"where fmt-newline\">where\n    &lt;Types as NodeType&gt;::BlockType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::BlockQueryData"]],["impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.BlockHeaderQueryData.html\" title=\"struct hotshot_query_service::availability::BlockHeaderQueryData\">BlockHeaderQueryData</a>&lt;Types&gt;<span class=\"where fmt-newline\">where\n    &lt;Types as NodeType&gt;::BlockType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::BlockHeaderQueryData"]],["impl&lt;Types, I&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/availability/enum.InconsistentBlockError.html\" title=\"enum hotshot_query_service::availability::InconsistentBlockError\">InconsistentBlockError</a>&lt;Types, I&gt;<span class=\"where fmt-newline\">where\n    &lt;Types as NodeType&gt;::BlockType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;I as NodeImplementation&lt;Types&gt;&gt;::Leaf: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::SignatureKey as SignatureKey&gt;::QCType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::InconsistentBlockError"]],["impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.TransactionQueryData.html\" title=\"struct hotshot_query_service::availability::TransactionQueryData\">TransactionQueryData</a>&lt;Types&gt;<span class=\"where fmt-newline\">where\n    &lt;Types as NodeType&gt;::BlockType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::BlockType as <a class=\"trait\" href=\"hotshot_query_service/availability/trait.QueryableBlock.html\" title=\"trait hotshot_query_service::availability::QueryableBlock\">QueryableBlock</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/availability/trait.QueryableBlock.html#associatedtype.InclusionProof\" title=\"type hotshot_query_service::availability::QueryableBlock::InclusionProof\">InclusionProof</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Transaction: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::query_data::TransactionQueryData"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.Options.html\" title=\"struct hotshot_query_service::availability::Options\">Options</a>",1,["hotshot_query_service::availability::Options"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/availability/enum.Error.html\" title=\"enum hotshot_query_service::availability::Error\">Error</a>",1,["hotshot_query_service::availability::Error"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.RequestSnafu.html\" title=\"struct hotshot_query_service::availability::RequestSnafu\">RequestSnafu</a>",1,["hotshot_query_service::availability::RequestSnafu"]],["impl&lt;__T0&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.QueryLeafSnafu.html\" title=\"struct hotshot_query_service::availability::QueryLeafSnafu\">QueryLeafSnafu</a>&lt;__T0&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::QueryLeafSnafu"]],["impl&lt;__T0&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.QueryBlockSnafu.html\" title=\"struct hotshot_query_service::availability::QueryBlockSnafu\">QueryBlockSnafu</a>&lt;__T0&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::QueryBlockSnafu"]],["impl&lt;__T0&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.QueryTransactionSnafu.html\" title=\"struct hotshot_query_service::availability::QueryTransactionSnafu\">QueryTransactionSnafu</a>&lt;__T0&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::QueryTransactionSnafu"]],["impl&lt;__T0&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.QueryProposalsSnafu.html\" title=\"struct hotshot_query_service::availability::QueryProposalsSnafu\">QueryProposalsSnafu</a>&lt;__T0&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::QueryProposalsSnafu"]],["impl&lt;__T0, __T1&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.InvalidTransactionIndexSnafu.html\" title=\"struct hotshot_query_service::availability::InvalidTransactionIndexSnafu\">InvalidTransactionIndexSnafu</a>&lt;__T0, __T1&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    __T1: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::InvalidTransactionIndexSnafu"]],["impl&lt;__T0, __T1&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.LeafStreamSnafu.html\" title=\"struct hotshot_query_service::availability::LeafStreamSnafu\">LeafStreamSnafu</a>&lt;__T0, __T1&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    __T1: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::LeafStreamSnafu"]],["impl&lt;__T0, __T1&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.BlockStreamSnafu.html\" title=\"struct hotshot_query_service::availability::BlockStreamSnafu\">BlockStreamSnafu</a>&lt;__T0, __T1&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    __T1: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::BlockStreamSnafu"]],["impl&lt;__T0, __T1&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/availability/struct.CustomSnafu.html\" title=\"struct hotshot_query_service::availability::CustomSnafu\">CustomSnafu</a>&lt;__T0, __T1&gt;<span class=\"where fmt-newline\">where\n    __T0: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    __T1: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::availability::CustomSnafu"]],["impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/struct.Iter.html\" title=\"struct hotshot_query_service::data_source::Iter\">Iter</a>&lt;'a, T&gt;",1,["hotshot_query_service::ledger_log::Iter"]],["impl&lt;Types, I, UserData&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/struct.QueryData.html\" title=\"struct hotshot_query_service::data_source::QueryData\">QueryData</a>&lt;Types, I, UserData&gt;<span class=\"where fmt-newline\">where\n    UserData: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::BlockType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;I as NodeImplementation&lt;Types&gt;&gt;::Leaf: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::SignatureKey as SignatureKey&gt;::QCType: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Types as NodeType&gt;::Transaction: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;&lt;Types as NodeType&gt;::BlockType as <a class=\"trait\" href=\"hotshot_query_service/availability/trait.QueryableBlock.html\" title=\"trait hotshot_query_service::availability::QueryableBlock\">QueryableBlock</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/availability/trait.QueryableBlock.html#associatedtype.TransactionIndex\" title=\"type hotshot_query_service::availability::QueryableBlock::TransactionIndex\">TransactionIndex</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["hotshot_query_service::data_source::QueryData"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/enum.Error.html\" title=\"enum hotshot_query_service::Error\">Error</a>",1,["hotshot_query_service::error::Error"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/status/struct.MempoolQueryData.html\" title=\"struct hotshot_query_service::status::MempoolQueryData\">MempoolQueryData</a>",1,["hotshot_query_service::status::query_data::MempoolQueryData"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/status/struct.Options.html\" title=\"struct hotshot_query_service::status::Options\">Options</a>",1,["hotshot_query_service::status::Options"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"hotshot_query_service/status/enum.Error.html\" title=\"enum hotshot_query_service::status::Error\">Error</a>",1,["hotshot_query_service::status::Error"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.73.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hotshot_query_service/struct.Options.html\" title=\"struct hotshot_query_service::Options\">Options</a>",1,["hotshot_query_service::Options"]]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()