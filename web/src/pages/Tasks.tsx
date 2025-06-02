import React, { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  CheckCircleIcon,
  ArrowLeftIcon,
  ClockIcon,
  ExclamationTriangleIcon,
  FireIcon,
  TagIcon,
  CalendarIcon,
  SparklesIcon,
} from "@heroicons/react/24/outline";
import type { VoiceSession, Task } from "../types";
import { fetchSessions } from "../services/api";
import { formatDate } from "../utils";

/**
 * Tasks page component that aggregates and displays all tasks from voice sessions
 * Features:
 * - Aggregated view of all tasks across sessions
 * - Priority-based styling and sorting
 * - Click to navigate to source recording
 * - Search and filter functionality
 */
const Tasks: React.FC = () => {
  const [sessions, setSessions] = useState<VoiceSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [priorityFilter, setPriorityFilter] = useState<string>("All");
  const navigate = useNavigate();

  // Load sessions from backend on component mount
  useEffect(() => {
    loadSessions();
  }, []);

  const loadSessions = async () => {
    try {
      setLoading(true);
      setError(null);
      const fetchedSessions = await fetchSessions({
        sort_by: 'timestamp',
        sort_order: 'desc'
      });
      setSessions(fetchedSessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load tasks. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  // Extract all tasks from sessions with source information
  const allTasks = sessions.flatMap(session => 
    (session.analysis?.tasks || []).map(task => ({
      id: `${session.id}-${task.title}`,
      ...task,
      sessionId: session.id,
      sessionTitle: session.title,
      date: session.timestamp
    }))
  );

  // Filter tasks based on search query and priority
  const filteredTasks = allTasks.filter(task => {
    const matchesSearch = task.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         (task.description && task.description.toLowerCase().includes(searchQuery.toLowerCase())) ||
                         task.sessionTitle.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesPriority = priorityFilter === "All" || task.priority === priorityFilter;
    return matchesSearch && matchesPriority;
  });

  // Sort tasks by priority (Urgent > High > Medium > Low)
  const priorityOrder = { 'Urgent': 4, 'High': 3, 'Medium': 2, 'Low': 1 };
  const sortedTasks = filteredTasks.sort((a, b) => 
    priorityOrder[b.priority] - priorityOrder[a.priority]
  );

  // Get priority styling
  const getPriorityStyle = (priority: string) => {
    switch (priority) {
      case 'Urgent':
        return 'bg-red-100 text-red-800 border-red-200';
      case 'High':
        return 'bg-orange-100 text-orange-800 border-orange-200';
      case 'Medium':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'Low':
        return 'bg-green-100 text-green-800 border-green-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getPriorityIcon = (priority: string) => {
    if (priority === 'Urgent' || priority === 'High') {
      return <ExclamationTriangleIcon className="w-4 h-4" />;
    }
    return <CheckCircleIcon className="w-4 h-4" />;
  };

  const priorities = ['All', 'Urgent', 'High', 'Medium', 'Low'];

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-indigo-50 to-purple-50">
      {/* Header */}
      <header className="bg-white/80 backdrop-blur-md shadow-sm border-b border-blue-200/50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 py-6">
          <div className="flex flex-col space-y-4">
            <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between space-y-4 sm:space-y-0">
              <div className="flex items-center space-x-4">
                <button
                  onClick={() => navigate('/')}
                  className="p-2 rounded-xl text-gray-600 hover:text-blue-600 hover:bg-blue-100 transition-all duration-200"
                >
                  <ArrowLeftIcon className="w-5 h-5" />
                </button>
                <div className="flex items-center space-x-3">
                  <div className="p-3 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-2xl shadow-lg">
                    <CheckCircleIcon className="w-7 h-7 text-white" />
                  </div>
                  <div>
                    <h1 className="text-2xl sm:text-3xl font-bold bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">Tasks</h1>
                    <p className="text-sm text-gray-600">{filteredTasks.length} action items from {sessions.length} recordings</p>
                  </div>
                </div>
              </div>
            </div>
            
            {/* Search and Filters */}
            <div className="flex flex-col sm:flex-row items-stretch sm:items-center space-y-3 sm:space-y-0 sm:space-x-4">
              {/* Priority Filter */}
              <select
                value={priorityFilter}
                onChange={(e) => setPriorityFilter(e.target.value)}
                className="px-4 py-3 rounded-xl border border-blue-200 bg-white/70 backdrop-blur-sm shadow-sm text-sm focus:ring-2 focus:ring-blue-400 focus:border-transparent outline-none transition-all duration-200"
              >
                {priorities.map(priority => (
                  <option key={priority} value={priority}>{priority} Priority</option>
                ))}
              </select>
              
              {/* Search */}
              <div className="relative flex-1 sm:flex-initial">
                <input
                  type="text"
                  placeholder="Search tasks..."
                  className="w-full sm:w-80 pl-4 pr-4 py-3 rounded-xl border border-blue-200 bg-white/70 backdrop-blur-sm shadow-sm text-sm focus:ring-2 focus:ring-blue-400 focus:border-transparent outline-none transition-all duration-200"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Content */}
      <main className="max-w-7xl mx-auto p-4 sm:p-6">
        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600">Loading tasks...</span>
          </div>
        )}

        {/* Error State */}
        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <div className="flex items-center">
              <svg className="w-5 h-5 text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
              </svg>
              <span className="text-red-800">{error}</span>
              <button 
                onClick={loadSessions}
                className="ml-auto text-red-600 hover:text-red-800 text-sm font-medium"
              >
                Retry
              </button>
            </div>
          </div>
        )}

        {/* Empty State */}
        {!loading && !error && filteredTasks.length === 0 && (
          <div className="text-center py-12">
            <CheckCircleIcon className="w-16 h-16 text-gray-300 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-gray-900 mb-2">No tasks found</h3>
            <p className="text-gray-500 mb-4">
              {searchQuery || priorityFilter !== "All" ? 'Try adjusting your search or filters.' : 'Start recording to capture your tasks!'}
            </p>
          </div>
        )}

        {/* Tasks Grid */}
        {!loading && !error && sortedTasks.length > 0 && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {sortedTasks.map((task, index) => {
              const getPriorityGradient = (priority: string) => {
                switch (priority) {
                  case 'Urgent': return 'from-red-400 to-pink-500';
                  case 'High': return 'from-orange-400 to-red-500';
                  case 'Medium': return 'from-yellow-400 to-orange-500';
                  case 'Low': return 'from-green-400 to-blue-500';
                  default: return 'from-gray-400 to-gray-500';
                }
              };
              
              const getPriorityBgGlow = (priority: string) => {
                switch (priority) {
                  case 'Urgent': return 'from-red-200/40 to-pink-200/40';
                  case 'High': return 'from-orange-200/40 to-red-200/40';
                  case 'Medium': return 'from-yellow-200/40 to-orange-200/40';
                  case 'Low': return 'from-green-200/40 to-blue-200/40';
                  default: return 'from-gray-200/40 to-gray-200/40';
                }
              };
              
              return (
                <div
                  key={task.id}
                  className="group relative cursor-pointer"
                  onClick={() => navigate(`/detail/${task.sessionId}`)}
                >
                  {/* Background glow effect */}
                  <div className={`absolute inset-0 bg-gradient-to-br ${getPriorityBgGlow(task.priority)} rounded-3xl blur-xl group-hover:blur-2xl transition-all duration-500 opacity-0 group-hover:opacity-100`}></div>
                  
                  {/* Card content */}
                  <div className="relative bg-white/90 backdrop-blur-sm border border-blue-200/60 rounded-3xl p-6 hover:shadow-2xl transition-all duration-300 group-hover:border-blue-300/80 group-hover:bg-white/95">
                    {/* Header with priority indicator */}
                    <div className="flex items-center justify-between mb-4">
                      <div className="flex items-center space-x-3">
                        <div className="relative">
                          <div className={`absolute inset-0 bg-gradient-to-br ${getPriorityGradient(task.priority)} rounded-2xl blur-md opacity-50 group-hover:opacity-70 transition-opacity`}></div>
                          <div className={`relative w-12 h-12 bg-gradient-to-br ${getPriorityGradient(task.priority)} rounded-2xl flex items-center justify-center shadow-lg`}>
                            {getPriorityIcon(task.priority)}
                          </div>
                        </div>
                        <div>
                          <span className={`text-xs font-semibold px-3 py-1 rounded-full bg-gradient-to-r ${getPriorityGradient(task.priority)} text-white shadow-sm`}>
                            {task.priority} Priority
                          </span>
                        </div>
                      </div>
                      <div className="p-2 rounded-full bg-gray-100 group-hover:bg-blue-100 transition-colors">
                        <SparklesIcon className="w-4 h-4 text-gray-400 group-hover:text-blue-500 transition-colors" />
                      </div>
                    </div>
                    
                    {/* Task content */}
                    <div className="mb-4">
                      <h3 className="text-gray-800 font-semibold text-lg mb-2 group-hover:text-gray-900 transition-colors">
                        {task.title}
                      </h3>
                      {task.description && (
                        <p className="text-gray-600 text-sm leading-relaxed mb-3">
                          {task.description}
                        </p>
                      )}
                    </div>
                    
                    {/* Due date if exists */}
                    {task.due_date && (
                      <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-xl">
                        <div className="flex items-center space-x-2 text-red-700">
                          <CalendarIcon className="w-4 h-4" />
                          <span className="text-sm font-medium">
                            Due: {new Date(task.due_date).toLocaleDateString()}
                          </span>
                        </div>
                      </div>
                    )}
                    
                    {/* Source information */}
                    <div className="flex items-center justify-between pt-4 border-t border-gray-100">
                      <div className="flex items-center space-x-3">
                        <div className="flex items-center space-x-1 text-sm text-gray-600">
                          <TagIcon className="w-4 h-4" />
                          <span className="font-medium truncate max-w-32">{task.sessionTitle}</span>
                        </div>
                        <div className="flex items-center space-x-1 text-sm text-gray-500">
                          <ClockIcon className="w-4 h-4" />
                          <span>{formatDate(new Date(task.date))}</span>
                        </div>
                      </div>
                      <div className="flex-shrink-0">
                        <div className={`w-8 h-8 rounded-full bg-gradient-to-r ${getPriorityGradient(task.priority)} flex items-center justify-center group-hover:scale-110 transition-transform`}>
                          <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                          </svg>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </main>
    </div>
  );
};

export default Tasks;